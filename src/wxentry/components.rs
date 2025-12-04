use crate::*;
use anyhow::bail;
use chrono_tz::Tz;
use derive_more::Display;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Station {
    pub name: String,
    pub altitude: Altitude,
    pub coords: Coordinates,
    pub time_zone: Tz,
}

impl Default for Station {
    fn default() -> Self {
        Station {
            name: "NULL Island".into(),
            altitude: Altitude::new(0.0, Meter),
            coords: (0.0, 0.0).into(),
            time_zone: Tz::UTC,
        }
    }
}

// #[derive(Clone, Debug, Serialize, Deserialize)]
// pub struct StaticStation {
//     pub name: &'static str,
//     pub altitude: Altitude,
//     pub coords: (f32, f32),
//     pub time_zone: Tz,
// }

// impl From<StaticStation> for Station {
//     fn from(value: StaticStation) -> Station {
//         Station {
//             name: value.name.into(),
//             altitude: value.altitude,
//             coords: value.coords.into(),
//             time_zone: value.time_zone,
//         }
//     }
// }

// pub trait StationRef: Deref<Target = Station> {}

// impl StationRef for &Station {}
// impl StationRef for Arc<Station> {}

// impl StationRef for StaticStation {}
// impl Deref for StaticStation {
//     type Target = Station;
//     fn deref(&self) -> &Station {
//         let s: Station = StaticStation::into(self.clone());
//         // let a: Arc<Station> = Arc::new(s);
//         s
//     }
// }

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Coordinates {
    pub latitude: f32,
    pub longitude: f32,
}

impl From<(f32, f32)> for Coordinates {
    fn from(value: (f32, f32)) -> Self {
        Coordinates {
            latitude: value.0,
            longitude: value.1,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum Param {
    Temperature,
    Pressure,
    Visibility,
    Dewpoint,
    RelativeHumidity,
    WindSpeed,
    WindDirection,
    Wind,
    SkyCover,
    WxCodes,
    RawMetar,
    PrecipToday,
    Precip,
    Altimeter,
    Cape,
}

// LAYER

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Layer {
    All,
    Indoor,
    NearSurface,
    SeaLevel,
    AGL(u64),  // in m
    MSL(u64),  // in m
    MBAR(u64), // in mb.
}
// we put the values in u64 because I really wanna be able to use hash on it
// and the precision isn't that important

use Layer::*;

impl Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            All => write!(f, ""),
            Indoor => write!(f, "Indoor"),
            NearSurface => write!(f, "Near Surface"),
            SeaLevel => write!(f, "Sea Level"),
            AGL(h) => write!(f, "{h} ft AGL"),
            MSL(h) => write!(f, "{h} ft MSL"),
            MBAR(p) => write!(f, "{p} mb"),
        }
    }
}

impl Layer {
    pub fn height_agl(&self, station_altitude: Altitude) -> Option<Altitude> {
        match self {
            All => Some(Altitude::new(f32::NAN, Meter)),
            Indoor => Some(Altitude::new(1., Meter)),
            NearSurface => Some(Altitude::new(2., Meter)),
            SeaLevel => Some(station_altitude * -1.),
            AGL(a) => Some(Altitude::new(*a as f32, Meter)),
            MSL(a) => Some(Altitude::new(*a as f32, Meter) - station_altitude),
            MBAR(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Wind {
    pub direction: Option<Direction>,
    pub speed: Speed,
}

impl Display for Wind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(dir) = self.direction {
            write!(f, "{}°@{}", dir.degrees(), self.speed)
        } else {
            write!(f, "{}", self.speed)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Display, PartialEq)]
pub enum CloudLayerCoverage {
    #[display(fmt = "FEW")]
    // #[serde(rename = "FEW")]
    Few,
    #[display(fmt = "SCT")]
    // #[serde(rename = "SCT")]
    Scattered,
    #[display(fmt = "BKN")]
    // #[serde(rename = "BKN")]
    Broken,
    #[display(fmt = "OVC")]
    // #[serde(rename = "OVC")]
    Overcast,
}

impl CloudLayerCoverage {
    pub fn str(&self) -> &'static str {
        match self {
            Self::Few => "FEW",
            Self::Scattered => "SCT",
            Self::Broken => "BKN",
            Self::Overcast => "OVC",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CloudLayer {
    pub coverage: CloudLayerCoverage,
    pub height: u32, // given in feet
}

impl CloudLayer {
    pub fn from_code(code: &str, height: u32) -> Result<Option<CloudLayer>> {
        let coverage_opt = match code {
            "SKC" => None,
            "CLR" => None,
            "FEW" => Some(CloudLayerCoverage::Few),
            "SCT" => Some(CloudLayerCoverage::Scattered),
            "BKN" => Some(CloudLayerCoverage::Broken),
            "OVC" => Some(CloudLayerCoverage::Overcast),
            _ => bail!("Unknown cloud cover string '{code}'"),
        };

        match coverage_opt {
            Some(coverage) => Ok(Some(CloudLayer { coverage, height })),
            None => Ok(None),
        }
    }
}

impl fmt::Display for CloudLayer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{} ft", self.coverage, self.height)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SkyCoverage {
    Clear,
    Cloudy(Vec<CloudLayer>),
}

impl fmt::Display for SkyCoverage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clear => write!(f, "CLR"),
            Self::Cloudy(v) => {
                write!(
                    f,
                    "{}",
                    v.iter()
                        .map(|x| x.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

impl SkyCoverage {
    pub fn oktas(&self) -> u8 {
        match self {
            Self::Clear => 0,
            Self::Cloudy(v) => v
                .iter()
                .map(|x| match x.coverage {
                    CloudLayerCoverage::Few => 1,
                    CloudLayerCoverage::Scattered => 3,
                    CloudLayerCoverage::Broken => 6,
                    CloudLayerCoverage::Overcast => 8,
                })
                .max()
                .unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Precip {
    pub unknown: PrecipAmount,
    pub rain: PrecipAmount,
    pub snow: PrecipAmount,
}

impl Display for Precip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Rain: {}, Snow: {}, Unknown: {}",
            self.rain, self.snow, self.unknown
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Wx {
    pub blowing: bool,
    pub freezing: bool,
    pub showers: bool,
    pub squalls: bool,
    pub thunderstorm: bool,
    pub fog: bool,
    pub smoke: bool,

    pub visibility_inhibitor: bool,

    // #[serde(skip_serializing_if = "Intensity::is_none")]
    pub rain: Intensity,
    // #[serde(skip_serializing_if = "Intensity::is_none")]
    pub snow: Intensity,
    // #[serde(skip_serializing_if = "Intensity::is_none")]
    pub falling_ice: Intensity,
    // #[serde(skip_serializing_if = "Intensity::is_none")]
    pub dust: Intensity,
    // #[serde(skip_serializing_if = "Intensity::is_none")]
    pub sand: Intensity,
    // #[serde(skip_serializing_if = "Intensity::is_none")]
    pub funnel_cloud: Intensity, // light: FC, heavy: Tornado
    // #[serde(skip_serializing_if = "Intensity::is_none")]
    pub unknown: Intensity,
}

impl Wx {
    pub fn none() -> Wx {
        use Intensity::None;
        Wx {
            blowing: false,
            freezing: false,
            showers: false,
            squalls: false,
            thunderstorm: false,
            visibility_inhibitor: false,
            fog: false,
            smoke: false,
            unknown: None,
            rain: None,
            snow: None,
            falling_ice: None,
            dust: None,
            sand: None,
            funnel_cloud: None,
        }
    }

    pub fn combine(self, other: Wx) -> Wx {
        Wx {
            blowing: self.blowing || other.blowing,
            freezing: self.freezing || other.freezing,
            showers: self.showers || other.showers,
            squalls: self.squalls || other.squalls,
            thunderstorm: self.thunderstorm || other.thunderstorm,
            visibility_inhibitor: self.visibility_inhibitor || other.visibility_inhibitor,
            fog: self.fog || other.fog,
            smoke: self.smoke || other.smoke,
            rain: self.rain.most_intense(other.rain),
            snow: self.snow.most_intense(other.snow),
            falling_ice: self.falling_ice.most_intense(other.falling_ice),
            dust: self.dust.most_intense(other.dust),
            sand: self.sand.most_intense(other.sand),
            funnel_cloud: self.funnel_cloud.most_intense(other.funnel_cloud),
            unknown: self.unknown.most_intense(other.unknown),
        }
    }

    pub fn parse_code(code: &str) -> Wx {
        let re = Regex::new(r"(-|\+|BC|BL|BR|DR|DS|DU|DZ|FC|FG|FU|FZ|GR|GS|HZ|IC|MI|NSW|PL|PO|PR|PY|RA|SA|SG|SH|SN|SQ|SS|TS|UP|VA|VC|/+)").unwrap();

        let matches: Vec<&str> = re.find_iter(code).map(|x| x.as_str()).collect();

        let mut wx = Wx::none();
        let general_intensity;

        if matches.contains(&"VC") {
            general_intensity = Intensity::Nearby;
        } else if matches.contains(&"-") {
            general_intensity = Intensity::Light;
        } else if matches.contains(&"+") {
            general_intensity = Intensity::Heavy;
        } else {
            general_intensity = Intensity::Medium;
        }

        wx.freezing = matches.contains(&"FZ");
        wx.showers = matches.contains(&"SH");
        wx.blowing = matches.contains(&"BL")
            || matches.contains(&"SS")
            || matches.contains(&"PO")
            || matches.contains(&"DS");
        wx.squalls = matches.contains(&"SQ");
        wx.thunderstorm = matches.contains(&"TS");
        wx.fog = matches.contains(&"BR") || matches.contains(&"FG");
        wx.smoke = matches.contains(&"FU") || matches.contains(&"HZ");

        if matches.contains(&"RA") {
            wx.rain = general_intensity
        } else if matches.contains(&"DZ") {
            wx.rain = Intensity::VeryLight
        }

        if matches.contains(&"DU") {
            wx.dust = general_intensity
        } else if matches.contains(&"DS") {
            wx.dust = Intensity::Heavy
        }

        if matches.contains(&"SA") || matches.contains(&"PO") {
            wx.sand = general_intensity
        } else if matches.contains(&"SS") {
            wx.sand = Intensity::Heavy
        }

        if matches.contains(&"PL") {
            wx.falling_ice = general_intensity
        } else if matches.contains(&"GR") {
            wx.sand = Intensity::Heavy
        }

        if matches.contains(&"UP") {
            wx.unknown = general_intensity
        }

        if matches.contains(&"SN")
            || matches.contains(&"GS")
            || matches.contains(&"IC")
            || matches.contains(&"SG")
        {
            wx.snow = general_intensity;
        }

        if matches.contains(&"FC") {
            wx.funnel_cloud = general_intensity;
        }

        // for now we'll intentionally ignore BC,DR,MI,PR,PY,NSW
        wx
    }
}

#[derive(Debug, Copy, Clone, Serialize, PartialEq, PartialOrd)]
pub enum Intensity {
    None,
    Nearby,
    VeryLight,
    Light,
    Medium,
    Heavy,
}

impl Intensity {
    pub fn is_none(&self) -> bool {
        self == &Self::None
    }
    pub fn most_intense(self, other: Intensity) -> Intensity {
        if self > other {
            self
        } else {
            other
        }
    }
}
