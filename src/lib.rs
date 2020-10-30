use actix_utils::mpsc;
use actix_web::dev::Server;
use actix_web::{
    error, get, post, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder,
};
use boundary::{get_osm_boundaries, Boundary};
use derive_more::Display;
use futures::StreamExt;
use location::Location;
use rayon::prelude::*;
use rstar::RTree;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::str::from_utf8;
use tracing::{instrument, subscriber::set_global_default, Subscriber};
use tracing_actix_web::TracingLogger;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub mod boundary;
pub mod location;
pub struct ServiceConfig {
    pub tree: RTree<Boundary>,
    pub port: u16,
    pub parallel: bool,
}

#[derive(Deserialize)]
struct LocateQuery {
    loc: Location,
}

#[derive(Serialize)]
struct LocateResponse {
    names: Vec<String>,
}

fn boundary_names(loc: &Location, tree: &RTree<Boundary>) -> Vec<String> {
    let point = loc.clone().into();
    let candidates: Vec<&Boundary> = tree.locate_all_at_point(&point).collect();
    candidates
        .into_iter()
        .filter(|boundary| boundary.contains(&point))
        .map(|boundary| boundary.name.clone())
        .collect()
}

#[derive(Debug, Display)]
struct ParsingError(String);

impl error::ResponseError for ParsingError {}

fn parse_loc_line_2(line: &str) -> Result<(&str, Location), ParsingError> {
    let parts: Vec<&str> = line.split(',').take(3).collect();
    if parts.len() != 3 {
        return Err(ParsingError(format!(
            "csv row needs to have 3 fields: \"id,lng,lat\" {}",
            line
        )));
    }
    let id = parts[0];
    let location = (|| -> Result<Location, Box<dyn std::error::Error>> {
        let lng: f64 = parts[1].parse()?;
        let lat: f64 = parts[2].parse()?;
        let location = Location::new(lng, lat)?;
        Ok(location)
    })()
    .map_err(|e| ParsingError(e.to_string()))?;
    Ok((id, location))
}

#[get("/health")]
async fn health(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().finish()
}

#[post("/bulk_stream")]
async fn bulk_stream(
    mut payload: web::Payload,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let (tx, rx_body) = mpsc::channel();
    let mut remainder: Option<String> = None;

    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        let utf8_str = match remainder {
            Some(prefix) => format!("{}{}", prefix, from_utf8(&chunk)?),
            None => from_utf8(&chunk)?.into(),
        };
        let mut lines: Vec<&str> = utf8_str.split('\n').collect();
        remainder = lines.pop().map(String::from);
        if data.parallel {
            let byte_vec = lines
                .par_iter()
                .map(|line| {
                    let output = process_line(line, &data.tree)?;
                    let bytes: web::Bytes = web::Bytes::from(output);
                    Ok(bytes)
                })
                .collect::<Result<Vec<_>, ParsingError>>()?;

            for bytes in byte_vec {
                let _ = tx.send(Ok::<_, Error>(bytes));
            }
        } else {
            for line in lines {
                let output = process_line(&line, &data.tree)?;
                let bytes = web::Bytes::from(output);
                let _ = tx.send(Ok::<_, Error>(bytes));
            }
        }
    }

    Ok(HttpResponse::Ok().streaming(rx_body))
}

#[instrument(skip(tree))]
fn process_line(line: &str, tree: &RTree<Boundary>) -> Result<String, ParsingError> {
    let (id, loc) = parse_loc_line_2(line)?;
    let names = boundary_names(&loc, tree);
    let output = format!("{},{}\n", id, names.join(","));
    Ok(output)
}

#[post("/bulk")]
async fn bulk(mut payload: web::Payload, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = payload.next().await {
        bytes.extend_from_slice(&item?);
    }
    let output_lines = web::block(move || -> Result<Vec<String>, ParsingError> {
        let process = |line: &&str| process_line(*line, &data.tree);
        let utf8_str = from_utf8(&bytes)
            .map_err(|_| ParsingError("could not parse payload into utf8 string".into()))?;
        let lines: Vec<&str> = utf8_str.split_terminator('\n').collect();
        if data.parallel {
            lines.par_iter().map(process).collect()
        } else {
            lines.iter().map(process).collect()
        }
    })
    .await?;
    let body: String = output_lines.into_iter().collect();

    Ok(HttpResponse::Ok().body(body))
}

#[get("/locate")]
async fn locate(query: web::Query<LocateQuery>, data: web::Data<Data>) -> impl Responder {
    let names = web::block(move || -> Result<_, ()> { Ok(boundary_names(&query.loc, &data.tree)) })
        .await
        .unwrap();
    web::Json(LocateResponse { names })
}

#[get("/locate_async")]
async fn locate_async(query: web::Query<LocateQuery>, data: web::Data<Data>) -> impl Responder {
    let names = boundary_names(&query.loc, &data.tree);
    web::Json(LocateResponse { names })
}

struct Data {
    tree: RTree<Boundary>,
    parallel: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;

    #[actix_rt::test]
    async fn health_check() {
        let mut app = test::init_service(App::new().service(health)).await;
        let req = test::TestRequest::get().uri("/health").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }
}

pub fn load_tree(
    path: PathBuf,
    admin_level: &[u8],
) -> Result<RTree<Boundary>, Box<dyn std::error::Error>> {
    let boundaries = get_osm_boundaries(path, admin_level)?;
    let tree = RTree::bulk_load(boundaries);
    Ok(tree)
}

fn get_subscriber(env_filter: String) -> impl Subscriber + Send + Sync {
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(PKG_NAME.into(), std::io::stdout);
    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger");
    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub fn run_service(config: ServiceConfig) -> Result<Server, std::io::Error> {
    std::env::set_var("RUST_LOG", "info,actix_web=error");
    let subscriber = get_subscriber("info".into());
    init_subscriber(subscriber);
    // env_logger::init();
    // let tree = load_tree(config.bin_path)?;
    let ServiceConfig {
        tree,
        port,
        parallel,
    } = config;
    let data = web::Data::new(Data { tree, parallel });
    log::info!("rtree loaded");
    let server = HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            // .wrap(Logger::default())
            .wrap(TracingLogger)
            .service(health)
            .service(locate)
            .service(locate_async)
            .service(bulk_stream)
            .service(bulk)
    })
    .bind(format!("127.0.0.1:{}", port))?
    .run();
    Ok(server)
}
