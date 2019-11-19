use boundary::get_osm_boundaries;
use boundary::Boundary;
use docopt::Docopt;
use rstar::RTree;
use serde::Deserialize;
use std::error::Error;
use std::fs::write;

pub mod boundary;

const USAGE: &'static str = "
build rtree of admin hierarchies

Usage:
  build-rtree --pbf=FILE --bin=FILE
  build-rtree (-h | --help)
  build-rtree --version

Options:
  -p FILE, --pbf=FILE  OSM protobuf input file
  -b FILE, --bin=FILE  RTree binary output file
  -h --help            Show this screen.
  --version            Show version.
";

#[derive(Debug, Deserialize)]
struct Args {
    flag_pbf: String,
    flag_bin: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    // let path = "germany-boundaries.pbf";
    let boundaries = get_osm_boundaries(args.flag_pbf)?;
    let tree = RTree::<Boundary>::bulk_load(boundaries);
    let encoded: Vec<u8> = bincode::serialize(&tree)?;
    write(args.flag_bin, encoded)?;
    Ok(())
}
