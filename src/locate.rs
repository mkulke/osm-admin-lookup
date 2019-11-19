use crate::geojson::write_geojson;
use boundary::Boundary;
use docopt::Docopt;
use rstar::RTree;
use serde::Deserialize;
use std::error::Error;
use std::fs::File;

pub mod boundary;
pub mod geojson;

const USAGE: &'static str = "
locate point in boundaries

Usage:
  locate <lng> <lat> --bin=FILE [--geojson=FILE]
  locate (-h | --help)
  locate --version

Options:
  -b FILE, --bin=<FILE>      The RTree binary input file
  -g FILE, --geojson=<FILE>  Create a geojson file with the location's features.
  -h --help                  Show this screen.
  --version                  Show version.
";

#[derive(Debug, Deserialize)]
struct Args {
    arg_lng: f64,
    arg_lat: f64,
    flag_geojson: Option<String>,
    flag_bin: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let file = File::open(args.flag_bin)?;
    let tree: RTree<Boundary> = bincode::deserialize_from(file)?;
    let point = &[args.arg_lng, args.arg_lat];
    let selected_boundaries: Vec<&Boundary> = tree
        .locate_all_at_point(point)
        .into_iter()
        .filter(|boundary| boundary.contains(point))
        .collect();

    for boundary in &selected_boundaries {
        println!("boundary: {:?}", boundary.name);
    }
    if let Some(path) = args.flag_geojson {
        write_geojson(path, selected_boundaries)?;
    }
    Ok(())
}
