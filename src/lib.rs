use boundary::get_osm_boundaries;
use boundary::Boundary;
use location::Location;
use std::error::Error;
use std::fs::File;
use std::io::ErrorKind;
use std::path::PathBuf;

pub mod boundary;
pub mod location;
pub mod service;

pub type RTree = rstar::RTree<Boundary>;

pub fn boundaries<'b>(loc: &Location, tree: &'b RTree) -> Vec<&'b Boundary> {
    let point = loc.clone().into();
    let candidates: Vec<&Boundary> = tree
        .locate_all_at_point(&point)
        .filter(|boundary| boundary.contains(&point))
        .collect();
    candidates
}

pub fn build_rtree(path: PathBuf, admin_levels: &[u8]) -> Result<RTree, Box<dyn Error>> {
    let boundaries = get_osm_boundaries(path, admin_levels)?;
    Ok(rstar::RTree::<Boundary>::bulk_load(boundaries))
}

pub fn load_tree(path: &PathBuf) -> Result<RTree, std::io::Error> {
    let file = File::open(path)?;
    let tree: RTree = bincode::deserialize_from(file).map_err(|e| {
        std::io::Error::new(
            ErrorKind::InvalidData,
            format!("could not deserialize rtree binary: {}", e),
        )
    })?;
    Ok(tree)
}
