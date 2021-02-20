use crate::geojson::write_geojson;
use boundary::Boundary;
use location::Location;
use rayon::prelude::*;
use rstar::RTree;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

pub mod boundary;
pub mod geojson;
pub mod location;

#[derive(Debug, StructOpt)]
#[structopt(name = "locate", about = "locate in rtree")]
struct Opt {
    /// rtree bin path
    #[structopt(short = "b", long = "bin")]
    bin_path: PathBuf,

    /// location (lng,lat)
    #[structopt(short = "l", long = "loc")]
    loc: Location,

    /// output geojson path
    #[structopt(short = "g", long = "geojson")]
    geojson_path: Option<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let file = File::open(opt.bin_path)?;
    let tree: RTree<Boundary> = bincode::deserialize_from(file)?;
    let point: &[f64; 2] = &opt.loc.into();
    let candidates: Vec<&Boundary> = tree.locate_all_at_point(&point).collect();
    let boundaries = candidates
        .into_par_iter()
        .filter(|boundary| boundary.contains(&point))
        .collect();

    match opt.geojson_path {
        Some(path) => write_geojson(File::open(path)?, boundaries)?,
        None => {
            for boundary in &boundaries {
                println!(
                    "boundary: {}, level: {}",
                    boundary.name, boundary.admin_level
                );
            }
        }
    }
    Ok(())
}
