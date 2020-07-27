use boundary::Boundary;
use easybench::bench;
use location::Location;
use rstar::RTree;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;

pub mod boundary;
pub mod geojson;
pub mod location;

#[derive(Debug, StructOpt)]
#[structopt(name = "build-rtree", about = "build rtree binary")]
struct Opt {
    /// rtree bin path
    #[structopt(short = "b", long = "bin")]
    bin_path: PathBuf,

    /// location (lng,lat)
    #[structopt(short = "l", long = "loc")]
    loc: Location,
}

fn locate_rtree<'a>(rtree: &'a RTree<Boundary>, point: &[f64; 2]) -> Vec<&'a Boundary> {
    rtree
        .locate_all_at_point(&point)
        .filter(|boundary| boundary.contains(&point))
        .collect()
}

fn locate_flat<'a>(rtree: &'a RTree<Boundary>, point: &[f64; 2]) -> Vec<&'a Boundary> {
    rtree
        .iter()
        .filter(|boundary| boundary.contains(&point))
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let file = File::open(opt.bin_path)?;
    let rtree: RTree<Boundary> = bincode::deserialize_from(file)?;
    let point = opt.loc.into();
    println!("rtree: {}", bench(|| locate_rtree(&rtree, &point)));
    println!("flat:  {}", bench(|| locate_flat(&rtree, &point)));

    Ok(())
}
