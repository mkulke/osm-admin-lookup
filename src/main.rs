use geojson::{Feature, GeoJson, Geometry, Value};
use osm_boundaries_utils::build_boundary;
use osmpbfreader::{OsmObj, OsmPbfReader};
use rstar::primitives::Rectangle;
use rstar::Envelope;
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use std::error::Error;
use std::fs::File;

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
    obj.is_relation()
        && obj.tags().contains("boundary", "administrative")
        && obj.tags().contains("admin_level", "9")
}

fn convert(mp: geo_types::MultiPolygon<f64>) -> geojson::Geometry {
    let point = geo_types::Point::new(2., 9.);
    // let v: geojson::Value = mp.into();
    let x = geojson::Value::from(&point);
    let x = geojson::Value::from(&point);
    unimplemented!();
}

fn main() -> Result<(), Box<dyn Error>> {
    let f = File::open("bremen-latest.osm.pbf")?;
    let mut pbf = OsmPbfReader::new(f);
    let tuples = pbf.get_objs_and_deps(is_admin)?;
    for tuple in tuples.clone() {
        let (_id, obj) = tuple;
        if let OsmObj::Relation(rel) = obj {
            let mp = build_boundary(&rel, &tuples).expect("fail");
            let v = Value::from(&mp);
            let g = Geometry::new(v);
            let gj = GeoJson::Feature(Feature {
                bbox: None,
                geometry: Some(g),
                id: None,
                properties: None,
                foreign_members: None,
            });
            let gjs = gj.to_string();
            println!("{:?}", gjs);
            break;
        }
    }
    // test_rtree();
    Ok(())
}
