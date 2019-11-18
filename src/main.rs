use geo::algorithm::bounding_rect::BoundingRect;
use geo_types::MultiPolygon;
use geojson::{Feature, Geometry, Value};
use num_traits::Float;
use osm_boundaries_utils::build_boundary;
use osmpbfreader::{OsmId, OsmObj, OsmPbfReader, Relation};
// use rayon::prelude::*;
use rstar::primitives::Rectangle;
use rstar::Envelope;
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use serde_json::map::Map;
use serde_json::to_value;
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::{write, File};

type Point2D = [f64; 2];

#[derive(Debug)]
struct Piece {
    rect: Rectangle<Point2D>,
    name: &'static str,
    area: f64,
}

impl Piece {
    pub fn new(lower: Point2D, upper: Point2D, name: &'static str) -> Self {
        let aabb = AABB::from_corners(lower, upper);
        let area = aabb.area();
        let rect = Rectangle::from_aabb(aabb);
        Piece { rect, name, area }
    }
}

impl RTreeObject for Piece {
    type Envelope = AABB<Point2D>;

    fn envelope(&self) -> Self::Envelope {
        self.rect.envelope()
    }
}

impl PointDistance for Piece {
    fn distance_2(&self, point: &Point2D) -> f64 {
        self.rect.distance_2(point)
    }
}

fn test_rtree() {
    let left_piece = Piece::new([0.0, 0.0], [0.4, 1.0], "left");
    let small_left_piece = Piece::new([0.0, 0.0], [0.3, 1.0], "small left");
    let right_piece = Piece::new([0.6, 0.0], [1.0, 1.0], "right");
    let middle_piece = Piece::new([0.25, 0.0], [0.75, 1.0], "middle");
    let huge_piece = Piece::new([0., 0.], [1.0, 1.0], "huge");

    let tree = RTree::<Piece>::bulk_load(vec![
        left_piece,
        small_left_piece,
        right_piece,
        middle_piece,
        huge_piece,
    ]);

    tree.locate_all_at_point(&[0.4, 0.5])
        .into_iter()
        .for_each(|p| {
            println!("piece: {:?}", p);
        });
}

fn is_admin(obj: &OsmObj) -> bool {
    get_admin(obj).is_some()
}

pub trait OsmObjExt {
    fn get_relation(&self) -> Option<&Relation>;
}

impl OsmObjExt for OsmObj {
    fn get_relation(&self) -> Option<&Relation> {
        match self {
            OsmObj::Relation(rel) => Some(rel),
            _ => None,
        }
    }
}

fn get_admin(obj: &OsmObj) -> Option<&Relation> {
    let rel = obj.get_relation()?;
    if obj.tags().contains("boundary", "administrative")
        && (obj.tags().contains("admin_level", "9") || obj.tags().contains("admin_level", "10"))
    {
        Some(rel)
    } else {
        None
    }
}

fn to_geometry<T>(mp: &MultiPolygon<T>) -> Geometry
where
    T: Float,
{
    let value = Value::from(mp);
    Geometry::new(value)
}

fn to_feature(name: &str, geometry: Geometry) -> Feature {
    let properties = match to_value(name) {
        Ok(value) => {
            let mut map = Map::new();
            map.insert("name".to_string(), value);
            Some(map)
        }
        _ => None,
    };

    Feature {
        bbox: None,
        geometry: Some(geometry),
        id: None,
        properties: properties,
        foreign_members: None,
    }
}

type OsmMap = BTreeMap<OsmId, OsmObj>;
fn get_btree(file: File) -> Result<OsmMap, Box<dyn Error>> {
    let mut pbf = OsmPbfReader::new(file);
    // let tuples = pbf.get_objs_and_deps(is_admin)?;

    let mut tuples = BTreeMap::new();
    for result in pbf.iter() {
        let obj = result?;
        tuples.insert(obj.id(), obj);
    }
    Ok(tuples)
}

fn main() -> Result<(), Box<dyn Error>> {
    // let file = File::open("hamburg-latest.osm.pbf")?;
    let file = File::open("berlin-regions.pbf")?;
    let btree = get_btree(file)?;

    let features = btree
        .values()
        .filter_map(get_admin)
        .filter_map(|rel| {
            let name = rel.tags.get("name")?;
            let boundary = build_boundary(&rel, &btree)?;
            Some((name, boundary))
        })
        .map(|(name, boundary)| {
            let geometry = to_geometry(&boundary);
            let _bounding_rect = boundary.bounding_rect();
            let feature = to_feature(name, geometry);
            feature
        })
        .collect();

    let feature_collection = geojson::FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };

    write("output.geojson", feature_collection.to_string())?;

    // test_rtree();
    Ok(())
}
