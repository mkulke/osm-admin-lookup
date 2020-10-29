use actix_utils::mpsc;
use actix_web::middleware::Logger;
use actix_web::{error, get, post, web, App, Error, HttpResponse, HttpServer, Responder};
use boundary::Boundary;
use derive_more::Display;
use futures::StreamExt;
use location::Location;
use rayon::prelude::*;
use rstar::RTree;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::PathBuf;
use std::str::from_utf8;
use structopt::StructOpt;

pub mod boundary;
pub mod location;

#[derive(Debug, StructOpt)]
#[structopt(name = "service", about = "locate in rtree")]
struct Opt {
    /// rtree bin path
    #[structopt(short = "b", long = "bin", env = "RTREE_BIN")]
    bin_path: PathBuf,
    /// parallel bulk processing
    #[structopt(short, long, env = "PARALLEL")]
    parallel: bool,
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
    let parts: Vec<&str> = line.split(",").take(3).collect();
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

fn process_line(line: &&str, tree: &RTree<Boundary>) -> Result<String, ParsingError> {
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
        let process = |line| process_line(line, &data.tree);
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info,actix_web=error");
    env_logger::init();
    let opt = Opt::from_args();
    let file = File::open(opt.bin_path)?;
    let tree: RTree<Boundary> = bincode::deserialize_from(file).expect("could not deserialize bin");
    let parallel = opt.parallel;
    let data = web::Data::new(Data { tree, parallel });
    log::info!("rtree loaded");
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(Logger::default())
            .service(locate)
            .service(locate_async)
            .service(bulk_stream)
            .service(bulk)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
