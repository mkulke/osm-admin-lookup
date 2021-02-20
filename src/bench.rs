use futures::StreamExt;
use location::Location;
use osm_admin_hierarchies::{run_service, ServiceConfig};
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::{self, BufRead};
use std::net::TcpListener;
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::time::Instant;

pub mod boundary;
pub mod geojson;
pub mod location;
pub mod service;

#[derive(StructOpt)]
#[structopt(about = "benchmark the service")]
enum Opt {
    Bulk {
        #[structopt(flatten)]
        common_opts: CommonOpts,
    },
    Single {
        /// block
        #[structopt(long = "block")]
        block: bool,

        #[structopt(flatten)]
        common_opts: CommonOpts,
    },
}

#[derive(StructOpt)]
struct CommonOpts {
    /// rtree bin path
    #[structopt(short = "b", long = "bin")]
    bin_path: PathBuf,

    /// locations csv path
    #[structopt(short = "l", long = "locs")]
    locations_path: PathBuf,

    /// iterations
    #[structopt(short = "i", long = "iterations", default_value = "3")]
    iterations: u32,

    /// max concurrency
    #[structopt(short = "m", long = "max", default_value = "4")]
    concurrency: u8,
}

fn spawn_app(path: PathBuf) -> String {
    let tree = service::load_tree(path).expect("could not build rtree");
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let config = ServiceConfig {
        tree,
        parallel: false,
        listener,
    };
    let server = run_service(config).expect("Failed to start server");
    let _ = tokio::spawn(server);
    format!("http://127.0.0.1:{}", port)
}

async fn perform_single_requests(
    base_url: &str,
    locations: impl Iterator<Item = Location>,
    concurrency: u8,
    block: bool,
) -> usize {
    let client = reqwest::Client::new();
    let fetches = futures::stream::iter(locations.into_iter().map(|location| {
        let client = &client;
        async move {
            let route = if block { "locate_with_block" } else { "locate" };
            let path = format!("{}/{}?loc={}", &base_url, route, location);
            let resp = client.get(&path).send().await?;
            if resp.status() != 200 {
                return Err("!= 200".into());
            }
            let _ = resp.text().await?;
            Ok(())
        }
    }))
    .buffer_unordered(concurrency.into())
    .collect::<Vec<Result<(), Box<dyn Error>>>>();
    fetches.await.into_iter().filter_map(|x| x.ok()).count()
}

async fn perform_bulk_request(base_url: &str, file: &mut File, concurrency: u8) -> usize {
    let client = &reqwest::Client::new();

    let bulks = futures::stream::iter(0..10)
        .map(|_: i32| {
            let mut data = Vec::new();
            file.read_to_end(&mut data).expect("could not read file");
            let route = format!("{}/bulk", &base_url);

            async move {
                let resp = client
                    .post(&route)
                    .body(data)
                    .send()
                    .await
                    .expect("request failed");
                if resp.status() != 200 {
                    return Err("!= 200".into());
                }
                let _ = resp.text().await?;
                Ok(())
            }
        })
        .buffer_unordered(concurrency.into())
        .collect::<Vec<Result<(), Box<dyn Error>>>>();

    bulks.await.into_iter().filter_map(|x| x.ok()).count()
}

async fn single(opts: CommonOpts, block: bool) {
    let base_url = spawn_app(opts.bin_path);

    for _ in 0..opts.iterations {
        let file = File::open(opts.locations_path.clone()).expect("cannot open locations file");
        let lines = io::BufReader::new(file).lines();
        let locations = lines.filter_map(|line| {
            let line = line.ok()?;
            let end_of_id_field = line.find(',')?;
            let loc_str = &line[end_of_id_field + 1..];
            let location: Location = loc_str.try_into().ok()?;
            Some(location)
        });

        let now = Instant::now();
        let oks = perform_single_requests(&base_url, locations, opts.concurrency, block).await;
        let new_now = Instant::now();
        println!(
            "took {:?} for {} requests",
            new_now.checked_duration_since(now).unwrap(),
            oks,
        );
    }
}

async fn bulk(opts: CommonOpts) {
    let base_url = spawn_app(opts.bin_path);

    for _ in 0..opts.iterations {
        let mut file = File::open(opts.locations_path.clone()).expect("cannot open locations file");
        let now = Instant::now();
        let oks = perform_bulk_request(&base_url, &mut file, opts.concurrency).await;
        let new_now = Instant::now();
        println!(
            "took {:?} for {} bulk requests",
            new_now.checked_duration_since(now).unwrap(),
            oks,
        );
    }
}

#[actix_rt::main]
async fn main() {
    let opt = Opt::from_args();
    match opt {
        Opt::Bulk { common_opts } => bulk(common_opts).await,
        Opt::Single { common_opts, block } => single(common_opts, block).await,
    };

    // single(opt).await;
    // bulk(opt).await;
}
