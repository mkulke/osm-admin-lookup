use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::contains::Contains;
use geo::polygon;
use geo_types::{MultiPolygon, Point};
use geojson::{Feature, Geometry, Value};
use osm_boundaries_utils::build_boundary;
use osmpbfreader::{OsmId, OsmObj, OsmPbfReader, Relation};
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
struct Boundary {
    rect: Rectangle<Point2D>,
    // name: &'static str,
    name: String,
    area: f64,
    mp: MultiPolygon<f64>,
}

struct AABBWrapper(AABB<Point2D>);
impl Into<MultiPolygon<f64>> for AABBWrapper {
    fn into(self) -> MultiPolygon<f64> {
        let [min_x, min_y] = self.0.lower();
        let [max_x, max_y] = self.0.upper();
        let poly_rect = polygon![
            (x: min_x, y: min_y),
            (x: min_x, y: max_y),
            (x: max_x, y: max_y),
            (x: max_x, y: min_y),
            (x: min_x, y: min_y),
        ];
        vec![poly_rect].into()
    }
}

impl Boundary {
    pub fn new(mp: MultiPolygon<f64>, name: String) -> Self {
        let rect = mp.bounding_rect().expect("yo");
        let lower = [rect.min.x, rect.min.y];
        let upper = [rect.max.x, rect.max.y];
        let aabb = AABB::from_corners(lower, upper);
        let area = aabb.area();
        let rect = Rectangle::from_aabb(aabb);
        Boundary {
            rect,
            name,
            area,
            mp,
        }
    }

    fn to_feature(&self) -> Feature {
        let properties = match to_value(&self.name) {
            Ok(value) => {
                let mut map = Map::new();
                map.insert("name".to_string(), value);
                Some(map)
            }
            _ => None,
        };

        let value = Value::from(&self.mp);
        let geometry = Geometry::new(value);

        Feature {
            bbox: None,
            geometry: Some(geometry),
            id: None,
            properties: properties,
            foreign_members: None,
        }
    }

    fn contains(&self, point: &Point2D) -> bool {
        let [x, y] = point;
        self.mp.contains(&Point::new(*x, *y))
    }
}

impl RTreeObject for Boundary {
    type Envelope = AABB<Point2D>;

    fn envelope(&self) -> Self::Envelope {
        self.rect.envelope()
    }
}

impl PointDistance for Boundary {
    fn distance_2(&self, point: &Point2D) -> f64 {
        self.rect.distance_2(point)
    }
}

fn get_test_boundaries() -> Vec<Boundary> {
    let boundaries = vec![
        ([0.0, 0.0], [0.4, 1.0], "left"),
        ([0.0, 0.0], [0.3, 1.0], "small left"),
        ([0.6, 0.0], [1.0, 1.0], "right"),
        ([0.25, 0.0], [0.75, 1.0], "middle"),
        ([0., 0.], [1.0, 1.0], "huge"),
    ]
    .iter()
    .map(|(lower, upper, name)| {
        let aabb = AABB::from_corners(*lower, *upper);
        let mp: MultiPolygon<f64> = AABBWrapper(aabb).into();
        Boundary::new(mp, name.to_string())
    })
    .collect();

    boundaries
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
    if !obj.tags().contains("boundary", "administrative") {
        return None;
    }
    if let Some(level) = obj.tags().get("admin_level") {
        match level.as_str() {
            "4" | "8" | "9" | "10" => Some(rel),
            _ => None,
        }
    } else {
        None
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

fn write_geojson(boundaries: Vec<&Boundary>) -> Result<(), std::io::Error> {
    let features = boundaries
        .iter()
        .map(|boundary| boundary.to_feature())
        .collect();

    let feature_collection = geojson::FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };

    write("output.geojson", feature_collection.to_string())
}

fn get_osm_boundaries() -> Result<Vec<Boundary>, Box<dyn Error>> {
    let file = File::open("germany-boundaries.pbf")?;
    // let file = File::open("berlin-regions.pbf")?;
    let btree = get_btree(file)?;

    let boundaries = btree
        .values()
        .filter_map(get_admin)
        .filter_map(|rel| {
            let name = rel.tags.get("name")?;
            let multi_polygon = build_boundary(&rel, &btree)?;
            Some((name, multi_polygon))
        })
        .map(|(name, multi_polygon)| Boundary::new(multi_polygon, name.to_string()))
        .collect();
    Ok(boundaries)
}

fn main() -> Result<(), Box<dyn Error>> {
    let boundaries = get_osm_boundaries()?;
    // let boundaries = get_test_boundaries();
    // write_geojson(boundaries)?;
    let tree = RTree::<Boundary>::bulk_load(boundaries);
    let point = &[13.371462, 52.570070];
    let selected_boundaries = tree
        .locate_all_at_point(point)
        .into_iter()
        .filter(|boundary| boundary.contains(point))
        .collect();
    write_geojson(selected_boundaries)?;
    Ok(())
}
