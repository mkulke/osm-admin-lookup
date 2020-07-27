use super::boundary::Boundary;
use geojson::{Feature, Geometry, Value};
use serde_json::map::Map;
use serde_json::to_value;
use std::io::Write;

impl Boundary {
    pub fn to_feature(&self) -> Feature {
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
            properties,
            foreign_members: None,
        }
    }
}

pub fn write_geojson(
    mut writer: impl Write,
    boundaries: Vec<&Boundary>,
) -> Result<(), std::io::Error> {
    let features = boundaries
        .iter()
        .map(|boundary| boundary.to_feature())
        .collect();

    let feature_collection = geojson::FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };

    writer.write(feature_collection.to_string().as_bytes())?;
    Ok(())
}
