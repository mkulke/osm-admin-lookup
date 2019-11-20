use crate::geojson::write_geojson;
use boundary::Boundary;
use clap::{crate_authors, crate_version, values_t, App, Arg};
use rstar::RTree;
use std::error::Error;
use std::fs::File;

pub mod boundary;
pub mod geojson;

fn is_float(val: String) -> Result<(), String> {
    match val.parse::<f64>() {
        Ok(_) => Ok(()),
        Err(_) => Err("Value is not a proper float.".to_string()),
    }
}

fn get_loc_arg<'a, 'b>(name: &'a str, short: &str) -> Arg<'a, 'b> {
    Arg::with_name(name)
        .required(true)
        .require_delimiter(true)
        .short(short)
        .long(name)
        .value_name("lon,lat")
        .validator(is_float)
        .number_of_values(2)
        .takes_value(true)
        .allow_hyphen_values(true)
}

fn get_file_arg<'a, 'b>(name: &'a str, short: &str, required: bool) -> Arg<'a, 'b> {
    Arg::with_name(name)
        .required(required)
        .short(short)
        .long(name)
        .value_name("FILE")
        .number_of_values(1)
        .takes_value(true)
}

struct Opts {
    bin_path: String,
    loc: [f64; 2],
    geojson_path: Option<String>,
}

fn get_cli_app<'a, 'b>() -> App<'a, 'b> {
    App::new("build rtree of admin hierarchies")
        .version(crate_version!())
        .author(crate_authors!())
        .arg(get_loc_arg("location", "l"))
        .arg(get_file_arg("bin", "b", true))
        .arg(get_file_arg("geojson", "g", false))
}

fn get_opts() -> Option<Opts> {
    let app = get_cli_app();
    let matches = app.get_matches();
    let bin_path = matches.value_of("bin")?.to_string();
    let mut loc = values_t!(matches.values_of("location"), f64).ok()?;
    let lat = loc.pop()?;
    let lng = loc.pop()?;
    let loc = [lng, lat];
    let geojson_path = matches.value_of("geojson").map(|v| v.to_string());
    let opts = Opts {
        bin_path,
        loc,
        geojson_path,
    };
    Some(opts)
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts = get_opts().unwrap();
    let file = File::open(opts.bin_path)?;
    let tree: RTree<Boundary> = bincode::deserialize_from(file)?;
    let point = opts.loc;
    let selected_boundaries: Vec<&Boundary> = tree
        .locate_all_at_point(&point)
        .filter(|boundary| boundary.contains(&point))
        .collect();

    for boundary in &selected_boundaries {
        println!("boundary: {:?}", boundary.name);
    }
    if let Some(path) = opts.geojson_path {
        write_geojson(path, selected_boundaries)?;
    }
    Ok(())
}
