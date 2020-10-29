use serde::de::Error as SerdeError;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::error::Error;
use std::num::ParseFloatError;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Location {
    lng: f64,
    lat: f64,
}

impl Location {
    pub fn new(lng: f64, lat: f64) -> Result<Location, &'static str> {
        if lng > 180.0 || lng < -180.0 {
            return Err("lng has to be a value between -180 & 180");
        }

        if lat > 90.0 || lat < -90.0 {
            return Err("lat has to be a value between -90 & 90");
        }

        Ok(Location { lng, lat })
    }
}

fn parse_loc_str(loc_str: &str) -> Result<Location, Box<dyn Error>> {
    let mut ll = loc_str.splitn(2, ',');
    match (ll.next(), ll.next()) {
        (Some(lng_str), Some(lat_str)) => {
            let lng = f64::from_str(lng_str)?;
            let lat = f64::from_str(lat_str)?;
            let location = Location::new(lng, lat)?;
            Ok(location)
        }
        _ => Err("location must be specified as 2 comma seperated floats".into()),
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        parse_loc_str(&string).map_err(SerdeError::custom)
    }
}

impl Serialize for Location {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Location", 2)?;
        state.serialize_field("lng", &self.lng)?;
        state.serialize_field("lat", &self.lat)?;
        state.end()
    }
}

impl From<Location> for [f64; 2] {
    fn from(loc: Location) -> Self {
        [loc.lng, loc.lat]
    }
}

impl FromStr for Location {
    type Err = ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s.split(',').collect();
        let lng = coords[0].parse::<f64>()?;
        let lat = coords[1].parse::<f64>()?;
        let loc = Self { lng, lat };
        Ok(loc)
    }
}
