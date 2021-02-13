use osm_admin_hierarchies::boundary::Boundary;
use osm_admin_hierarchies::{run_service, ServiceConfig};
use rstar::RTree;
use std::fs::File;
use std::io::{Error, ErrorKind};
use std::net::TcpListener;
use std::path::PathBuf;
use structopt::StructOpt;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

const PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, StructOpt)]
#[structopt(name = "service", about = "locate in rtree")]
pub struct Opt {
    /// rtree bin path
    #[structopt(short = "b", long = "bin", env = "RTREE_BIN")]
    pub bin_path: PathBuf,
    /// parallel bulk processing
    #[structopt(short = "P", long, env = "PARALLEL")]
    pub parallel: bool,
    /// http port
    #[structopt(short, long, env = "PORT", default_value = "8080")]
    pub port: u16,
}

pub fn load_tree(path: PathBuf) -> Result<RTree<Boundary>, Error> {
    let file = File::open(path)?;
    let tree: RTree<Boundary> = bincode::deserialize_from(file).map_err(|e| {
        Error::new(
            ErrorKind::InvalidData,
            format!("could not deserialize rtree binary: {}", e.to_string()),
        )
    })?;
    Ok(tree)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // init logger
    std::env::set_var("RUST_LOG", "info,actix_web=error");
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new(PKG_NAME.into(), std::io::stdout);

    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Failed to set subscriber");
    LogTracer::init().expect("Failed to set logger");

    // init tracer
    let (_tracer, _uninstall) = opentelemetry_jaeger::new_pipeline()
        .with_service_name(PKG_NAME)
        .install()
        .expect("jaeger pipeline install failed");

    let opt = Opt::from_args();
    let tree = load_tree(opt.bin_path)?;
    log::info!("rtree loaded");

    let Opt { parallel, port, .. } = opt;
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
    let config = ServiceConfig {
        tree,
        parallel,
        listener,
    };

    run_service(config)?.await
}
