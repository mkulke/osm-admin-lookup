use osm_admin_lookup::build_rtree;
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
    let rtree = build_rtree(
        opt.pbf_path,
        &opt.admin_level.unwrap_or_else(|| vec![4, 6, 8, 9, 10]),
    )?;
    let encoded: Vec<u8> = bincode::serialize(&rtree)?;
    write(opt.bin_path, encoded)?;
    Ok(())
}
