use std::{
    f32::consts::PI,
    fmt::{Display, Formatter},
    fs::File,
    io::BufReader,
    path::Path,
};

use exif::{Exif, In, Reader, Tag};
use regex::Regex;

/// Struct representing a geo location
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GeoLocation {
    pub latitude: f32,
    pub longitude: f32,
}

/// Display trait implementation for GeoLocation
impl Display for GeoLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[lat={} lon={}]", self.latitude, self.longitude,)
    }
}

impl GeoLocation {
    ///Computes the distance in meters using the Haversine formula
    pub fn distance_to(&self, other: &GeoLocation) -> f32 {
        let r = 6378137.; // radius of earth in meters
        let d_lat = other.latitude * PI / 180. - self.latitude * PI / 180.;
        let d_lon = other.longitude * PI / 180. - self.longitude * PI / 180.;
        let a = f32::sin(d_lat / 2.) * f32::sin(d_lat / 2.)
            + f32::cos(self.latitude * PI / 180.)
                * f32::cos(other.latitude * PI / 180.)
                * f32::sin(d_lon / 2.)
                * f32::sin(d_lon / 2.);
        let c = 2. * f32::atan2(f32::sqrt(a), f32::sqrt(1. - a));
        return r * c;
    }
}

/// Converts Degrees Minutes Seconds To Decimal Degrees
fn dms_to_dd(dms_string: &str, dms_ref: &str) -> Option<f32> {
    // Depending on the dms ref the value has to be multiplied by -1
    let dms_ref_multiplier = match dms_ref {
        "S" | "W" => -1.0,
        _ => 1.0,
    };

    let dms_parse_pattern: Regex = Regex::new(
        // e.g.: 7 deg 33 min 55.5155 sec or 7 deg 33 min 55 sec
        r"(?P<deg>\d+\.?\d*) deg (?P<min>\d+) min (?P<sec>\d+\.?\d*) sec",
    )
    .unwrap();
    let Some(pattern_match) = dms_parse_pattern.captures(dms_string) else {
        return None;
    };

    let Some(deg) = pattern_match
        .name("deg")
        .map(|cap| cap.as_str().parse::<f32>().unwrap())
    else {
        return None;
    };
    let Some(min) = pattern_match
        .name("min")
        .map(|cap| cap.as_str().parse::<f32>().unwrap())
    else {
        return None;
    };
    let Some(sec) = pattern_match
        .name("sec")
        .map(|cap| cap.as_str().parse::<f32>().unwrap())
    else {
        return None;
    };

    Some(dms_ref_multiplier * (deg + (min / 60.0) + (sec / 3600.0)))
}

impl GeoLocation {
    /// Detects the location from the exif data
    /// If the location is not found, the location is set to None
    fn from_exif(exif_data: &Exif) -> Option<GeoLocation> {
        let Some(latitude) = exif_data.get_field(Tag::GPSLatitude, In::PRIMARY) else {
            return None;
        };
        let Some(latitude_ref) = exif_data.get_field(Tag::GPSLatitudeRef, In::PRIMARY) else {
            return None;
        };
        let Some(longitude) = exif_data.get_field(Tag::GPSLongitude, In::PRIMARY) else {
            return None;
        };
        let Some(longitude_ref) = exif_data.get_field(Tag::GPSLongitudeRef, In::PRIMARY) else {
            return None;
        };
        let Some(dd_lat) = dms_to_dd(
            &latitude.display_value().to_string(),
            &latitude_ref.display_value().to_string(),
        ) else {
            return None;
        };
        let Some(dd_lon) = dms_to_dd(
            &longitude.display_value().to_string(),
            &longitude_ref.display_value().to_string(),
        ) else {
            return None;
        };

        Some(GeoLocation {
            latitude: dd_lat,
            longitude: dd_lon,
        })
    }
}

pub fn exif_geo_distance(path: &Path, reference: &GeoLocation) -> Option<f32> {
    let Ok(file) = File::open(path) else {
        return None;
    };
    let Ok(exif) = Reader::new().read_from_container(&mut BufReader::new(&file)) else {
        return None;
    };
    let Some(location) = GeoLocation::from_exif(&exif) else {
        return None;
    };
    let distance_meter = location.distance_to(&reference);
    return Some(distance_meter);
}
