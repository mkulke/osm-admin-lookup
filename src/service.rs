use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpServer, Responder};
use boundary::Boundary;
use location::Location;
use rstar::RTree;
use serde::Deserialize;
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

pub mod boundary;
pub mod location;

#[derive(Debug, StructOpt)]
#[structopt(name = "service", about = "locate in rtree")]
struct Opt {
    /// rtree bin path
    #[structopt(short = "b", long = "bin", env = "RTREE_BIN")]
    bin_path: PathBuf,
}

#[get("/{id}/{name}/index.html")]
async fn index(web::Path((id, name)): web::Path<(u32, String)>) -> impl Responder {
    format!("Hello {}! id:{}", name, id)
}

#[derive(Deserialize)]
struct LocateQuery {
    loc: Location,
}

#[get("/locate")]
async fn locate(query: web::Query<LocateQuery>, data: web::Data<RTree<Boundary>>) -> String {
    let boundaries = web::block(move || -> Result<String, ()> {
        let point = query.loc.clone().into();
        let candidates: Vec<&Boundary> = data.locate_all_at_point(&point).collect();
        let boundaries: Vec<String> = candidates
            .into_iter()
            .filter(|boundary| boundary.contains(&point))
            .map(|boundary| boundary.name.clone())
            .collect();
        Ok(boundaries.join(", "))
    })
    .await
    .unwrap();
    format!("bla {}", boundaries)
}

#[get("/locate_async")]
async fn locate_async(query: web::Query<LocateQuery>, data: web::Data<RTree<Boundary>>) -> String {
    let point = query.loc.clone().into();
    let candidates: Vec<&Boundary> = data.locate_all_at_point(&point).collect();
    let boundaries: Vec<String> = candidates
        .into_iter()
        .filter(|boundary| boundary.contains(&point))
        .map(|boundary| boundary.name.clone())
        .collect();
    format!("bla {}", boundaries.join(","))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info,actix_web=error");
    env_logger::init();
    let opt = Opt::from_args();
    let file = File::open(opt.bin_path)?;
    let tree: RTree<Boundary> = bincode::deserialize_from(file).expect("could not deserialize bin");
    let data = web::Data::new(tree);
    log::info!("rtree loaded");
    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(Logger::default())
            .service(index)
            .service(locate)
            .service(locate_async)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
