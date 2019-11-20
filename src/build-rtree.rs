use boundary::get_osm_boundaries;
use boundary::Boundary;
use clap::{crate_authors, crate_version, values_t, App, Arg};
use rstar::RTree;
use std::error::Error;
use std::fs::write;

pub mod boundary;

fn is_admin_level(al: String) -> Result<(), String> {
    if let Ok(num) = al.parse::<u8>() {
        if num <= 10 {
            return Ok(());
        }
    }
    Err("Value is not a proper float.".to_string())
}

fn get_cli_app<'a, 'b>() -> App<'a, 'b> {
    let file_arg = |name, short| {
        Arg::with_name(name)
            .required(true)
            .short(short)
            .long(name)
            .value_name("FILE")
            .number_of_values(1)
            .takes_value(true)
    };

    let admin_levels_arg = Arg::with_name("admin-levels")
        .required(true)
        .require_delimiter(true)
        .short("a")
        .long("admin-levels")
        .value_name("admin levels")
        .default_value("4,6,8,9,10")
        .validator(is_admin_level)
        .min_values(1)
        .max_values(10)
        .takes_value(true);

    App::new("build rtree of admin hierarchies")
        .version(crate_version!())
        .author(crate_authors!())
        .arg(admin_levels_arg)
        .arg(file_arg("pbf", "p"))
        .arg(file_arg("bin", "b"))
}

struct Opts {
    pbf_path: String,
    bin_path: String,
    admin_levels: Vec<String>,
}

fn get_opts() -> Option<Opts> {
    let app = get_cli_app();
    let matches = app.get_matches();
    let pbf_path = matches.value_of("pbf")?.to_string();
    let bin_path = matches.value_of("bin")?.to_string();
    let admin_levels = values_t!(matches.values_of("admin-levels"), String).ok()?;
    let opts = Opts {
        pbf_path,
        bin_path,
        admin_levels,
    };
    Some(opts)
}

fn main() -> Result<(), Box<dyn Error>> {
    let opts = get_opts().unwrap();
    let boundaries = get_osm_boundaries(opts.pbf_path, &opts.admin_levels)?;
    let tree = RTree::<Boundary>::bulk_load(boundaries);
    let encoded: Vec<u8> = bincode::serialize(&tree)?;
    write(opts.bin_path, encoded)?;
    Ok(())
}
