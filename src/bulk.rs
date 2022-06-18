use boundary::Boundary;
use rstar::RTree;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::io::{self, BufRead};
use std::path::PathBuf;
use structopt::StructOpt;

pub mod boundary;
pub mod geojson;

#[derive(Serialize, Deserialize)]
struct Input {
    id: String,
    loc: [f64; 2],
}

#[derive(Debug, StructOpt)]
#[structopt(name = "bulk", about = "bulk resolve lines from stdin")]
struct Opt {
    /// output bin path
    #[structopt(short = "b", long = "bin")]
    bin_path: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct Output {
    pub id: String,
    pub boundary_name: String,
    pub admin_level: u8,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let file = File::open(opt.bin_path)?;
    let tree: RTree<Boundary> = bincode::deserialize_from(file)?;
    let lines = io::BufReader::new(io::stdin()).lines();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in lines {
        let line = line?;
        let input: Input = serde_json::from_str(&line)?;
        let candidates = tree.locate_all_at_point(&input.loc);
        let boundaries = candidates.filter_map(|boundary| {
            if !boundary.contains(&input.loc) {
                return None;
            }
            let output = json!({
                "id": input.id,
                "boundary_name": boundary.name,
                "admin_level": boundary.admin_level,
            });
            Some(output)
        });
        for boundary in boundaries {
            writeln!(out, "{}", boundary)?;
        }
    }
    Ok(())
}
