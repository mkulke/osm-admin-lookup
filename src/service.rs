use rs_geo_playground::boundary::Boundary;
use rs_geo_playground::{run_service, ServiceConfig};
use rstar::RTree;
use std::fs::File;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use structopt::StructOpt;

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

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let opt = Opt::from_args();
    let tree = load_tree(opt.bin_path)?;
    let Opt { parallel, port, .. } = opt;
    let config = ServiceConfig {
        tree,
        parallel,
        port,
    };

    run_service(config)?.await
}
