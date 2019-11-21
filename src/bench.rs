use boundary::Boundary;
use clap::{crate_authors, crate_version, values_t, App, Arg};
use easybench::bench;
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

struct Opts {
    bin_path: String,
    loc: [f64; 2],
}

fn get_cli_app<'a, 'b>() -> App<'a, 'b> {
    let bin_arg = Arg::with_name("bin")
        .required(true)
        .short("b")
        .long("bin")
        .value_name("FILE")
        .number_of_values(1)
        .takes_value(true);

    let loc_arg = Arg::with_name("location")
        .required(true)
        .require_delimiter(true)
        .short("l")
        .long("location")
        .value_name("lon,lat")
        .validator(is_float)
        .number_of_values(2)
        .takes_value(true)
        .allow_hyphen_values(true);

    App::new("build rtree of admin hierarchies")
        .version(crate_version!())
        .author(crate_authors!())
        .arg(loc_arg)
        .arg(bin_arg)
}

fn get_opts() -> Option<Opts> {
    let app = get_cli_app();
    let matches = app.get_matches();
    let bin_path = matches.value_of("bin")?.to_string();
    let mut loc = values_t!(matches.values_of("location"), f64).ok()?;
    let lat = loc.pop()?;
    let lng = loc.pop()?;
    let loc = [lng, lat];
    let opts = Opts { bin_path, loc };
    Some(opts)
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
    let opts = get_opts().unwrap();
    let file = File::open(opts.bin_path)?;
    let rtree: RTree<Boundary> = bincode::deserialize_from(file)?;
    let point = opts.loc;
    println!("rtree: {}", bench(|| locate_rtree(&rtree, &point)));
    println!("flat:  {}", bench(|| locate_flat(&rtree, &point)));

    Ok(())
}
