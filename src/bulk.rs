use boundary::Boundary;
use rstar::RTree;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;
use structopt::StructOpt;

pub mod boundary;
pub mod geojson;

#[derive(Debug, StructOpt)]
#[structopt(name = "bulk", about = "bulk resolve lines from stdin")]
struct Opt {
    /// output bin path
    #[structopt(short = "b", long = "bin")]
    bin_path: PathBuf,
}

fn read_lines() -> io::Result<io::Lines<io::BufReader<io::Stdin>>> {
    let file = io::stdin();
    Ok(io::BufReader::new(file).lines())
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let file = File::open(opt.bin_path)?;
    let tree: RTree<Boundary> = bincode::deserialize_from(file)?;
    let lines = read_lines()?;
    for line in lines {
        let line = line?;
        let lng_lat: Vec<f64> = line.split(",").map(|s| s.parse().unwrap()).collect();
        let loc: [f64; 2] = [lng_lat[1], lng_lat[2]];
        let id = lng_lat[0];
        let candidates: Vec<&Boundary> = tree.locate_all_at_point(&loc.into()).collect();
        let boundaries: Vec<&Boundary> = candidates
            .into_iter()
            // .into_par_iter()
            .filter(|boundary| boundary.contains(&loc.into()))
            .collect();
        for boundary in boundaries {
            println!(
                "id: {}, boundary: {}, level: {}",
                id, boundary.name, boundary.admin_level
            );
        }
    }
    Ok(())
}
