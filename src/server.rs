use osm_admin_lookup::load_tree;
use osm_admin_lookup::service::start;
use std::path::PathBuf;
use structopt::StructOpt;
use tracing::info;

#[derive(Debug, StructOpt)]
#[structopt(name = "service", about = "locate in rtree")]
pub struct Opt {
    /// rtree bin path
    #[structopt(short = "b", long = "bin", env = "RTREE_BIN")]
    pub bin_path: PathBuf,
    /// http port
    #[structopt(short, long, env = "PORT", default_value = "8080")]
    pub port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    let rtree = load_tree(&opt.bin_path)?;
    info!("rtree {:?} loaded", opt.bin_path);
    start(rtree, opt.port).await?;
    Ok(())
}
