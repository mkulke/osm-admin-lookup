use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::contains::Contains;
use geo_types::{MultiPolygon, Point};
use osm_boundaries_utils::build_boundary;
use osmpbfreader::{OsmId, OsmObj, OsmPbfReader, Relation};
use rstar::primitives::Rectangle;
use rstar::Envelope;
use rstar::{PointDistance, RTreeObject, AABB};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

type Point2D = [f64; 2];

#[derive(Serialize, Deserialize, Debug)]
pub struct Boundary {
    rect: Rectangle<Point2D>,
    pub name: String,
    pub admin_level: u8,
    area: f64,
    pub mp: MultiPolygon<f64>,
}

impl Boundary {
    pub fn new(mp: MultiPolygon<f64>, name: &str, admin_level: u8) -> Self {
        let rect = mp.bounding_rect().expect("yo");
        let lower = [rect.min().x, rect.min().y];
        let upper = [rect.max().x, rect.max().y];
        let aabb = AABB::from_corners(lower, upper);
        let area = aabb.area();
        let rect = Rectangle::from_aabb(aabb);
        let name = name.to_string();
        Boundary {
            rect,
            name,
            area,
            admin_level,
            mp,
        }
    }

    pub fn contains(&self, point: &Point2D) -> bool {
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

fn get_admin<'a>(obj: &'a OsmObj, admin_levels: &[u8]) -> Option<&'a Relation> {
    let rel = obj.get_relation()?;
    if !obj.tags().contains("boundary", "administrative") {
        return None;
    }
    let level: u8 = obj.tags().get("admin_level")?.parse().ok()?;
    if !admin_levels.contains(&level) {
        return None;
    }
    Some(rel)
}

type OsmMap = BTreeMap<OsmId, OsmObj>;
fn get_btree(file: File) -> Result<OsmMap, Box<dyn Error>> {
    let mut pbf = OsmPbfReader::new(file);

    let mut tuples = BTreeMap::new();
    for result in pbf.iter() {
        let obj = result?;
        tuples.insert(obj.id(), obj);
    }
    Ok(tuples)
}

pub fn get_osm_boundaries(
    path: PathBuf,
    admin_levels: &[u8],
) -> Result<Vec<Boundary>, Box<dyn Error>> {
    let file = File::open(path)?;
    let btree = get_btree(file)?;

    let boundaries = btree
        .values()
        .filter_map(|obj| get_admin(obj, admin_levels))
        .filter_map(|rel| {
            let name = rel.tags.get("name")?;
            let admin_level = rel.tags.get("admin_level")?.parse().ok()?;
            let multi_polygon = build_boundary(rel, &btree)?;
            let boundary = Boundary::new(multi_polygon, name, admin_level);
            Some(boundary)
        })
        .collect();
    Ok(boundaries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::polygon;
    use rstar::RTree;

    struct AABBWrapper(AABB<Point2D>);
    impl From<AABBWrapper> for MultiPolygon<f64> {
        fn from(aabb: AABBWrapper) -> Self {
            let [min_x, min_y] = aabb.0.lower();
            let [max_x, max_y] = aabb.0.upper();
            polygon![
                (x: min_x, y: min_y),
                (x: min_x, y: max_y),
                (x: max_x, y: max_y),
                (x: max_x, y: min_y),
                (x: min_x, y: min_y),
            ]
            .into()
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
            Boundary::new(mp, name, 0)
        })
        .collect();

        boundaries
    }

    fn locate(rtree: &RTree<Boundary>, point: &Point2D) -> Vec<String> {
        rtree
            .locate_all_at_point(point)
            .map(|boundary| boundary.name.clone())
            .collect()
    }

    #[test]
    fn locates_points_in_boundaries() {
        let boundaries = get_test_boundaries();
        let rtree = RTree::<Boundary>::bulk_load(boundaries);
        let names = locate(&rtree, &[0.3, 0.2]);
        assert_eq!(names, ["huge", "middle", "small left", "left"]);
        let names = locate(&rtree, &[0.5, 0.5]);
        assert_eq!(names, ["huge", "middle"]);
        let names = locate(&rtree, &[0.8, 0.5]);
        assert_eq!(names, ["huge", "right"]);
        let names = locate(&rtree, &[1.1, 0.5]);
        let empty: Vec<String> = vec![];
        assert_eq!(names, empty);
    }
}
