use boundary::Boundary;
use rstar::RTree;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;
use std::fs::File;
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
    for line in lines {
        let line = line?;
        let input: Input = serde_json::from_str(&line)?;
        let candidates: Vec<&Boundary> = tree.locate_all_at_point(&input.loc).collect();
        let boundaries: Vec<&Boundary> = candidates
            .into_iter()
            // .into_par_iter()
            .filter(|boundary| boundary.contains(&input.loc))
            .collect();
        for boundary in boundaries {
            let output = json!({
                "id": input.id,
                "boundary_name": boundary.name,
                "admin_level": boundary.admin_level,
            });
            println!("{}", output);
        }
    }
    Ok(())
}
