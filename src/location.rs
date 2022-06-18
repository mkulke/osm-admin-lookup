use serde::de::Error as SerdeError;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::convert::{TryFrom, TryInto};
// use std::error::Error;
use std::fmt;
use std::num::ParseFloatError;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct Location {
    lng: f64,
    lat: f64,
}

impl Location {
    pub fn new(lng: f64, lat: f64) -> Result<Location, &'static str> {
        if !(-180.0..=180.0).contains(&lng) {
            return Err("lng has to be a value between -180 & 180");
        }

        if !(-90.0..=90.0).contains(&lat) {
            return Err("lat has to be a value between -90 & 90");
        }

        Ok(Location { lng, lat })
    }
}

impl TryFrom<&str> for Location {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let error_msg = "location must be specified as 2 comma seperated floats";
        let ll: Vec<&str> = value.split(',').collect();
        if ll.len() != 2 {
            return Err(error_msg);
        }
        let lng_str = ll.get(0).ok_or(error_msg)?;
        let lat_str = ll.get(1).ok_or(error_msg)?;
        let lng = f64::from_str(lng_str).or(Err(error_msg))?;
        let lat = f64::from_str(lat_str).or(Err(error_msg))?;
        Location::new(lng, lat).or(Err(error_msg))
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        string.as_str().try_into().map_err(SerdeError::custom)
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

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{},{}", self.lng, self.lat)
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
