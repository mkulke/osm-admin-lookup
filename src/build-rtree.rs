use boundary::get_osm_boundaries;
use boundary::Boundary;
use rstar::RTree;
use std::error::Error;
use std::fs::write;
use std::path::PathBuf;
use structopt::StructOpt;

pub mod boundary;

#[derive(Debug, StructOpt)]
#[structopt(name = "build-rtree", about = "build rtree binary")]
struct Opt {
    /// input osm PBF path
    #[structopt(short = "p", long = "pbf")]
    pbf_path: PathBuf,

    /// output bin path
    #[structopt(short = "b", long = "bin")]
    bin_path: PathBuf,

    /// admin level to consider
    #[structopt(short = "a", long = "admin-level")]
    admin_level: Option<Vec<u8>>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let boundaries = get_osm_boundaries(
        opt.pbf_path,
        &opt.admin_level.unwrap_or_else(|| vec![4, 6, 8, 9, 10]),
    )?;
    let tree = RTree::<Boundary>::bulk_load(boundaries);
    let encoded: Vec<u8> = bincode::serialize(&tree)?;
    write(opt.bin_path, encoded)?;
    Ok(())
}
