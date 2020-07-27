use std::num::ParseFloatError;
use std::str::FromStr;

#[derive(Debug)]
pub struct Location {
    lng: f64,
    lat: f64,
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
