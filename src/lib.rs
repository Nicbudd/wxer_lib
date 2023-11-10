use std::fmt::Display;

use serde::{Serialize, Deserialize};


#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Temperature(f32); // stored as F

impl Temperature {
    pub fn celsius(&self) -> f32 {
        (self.0 * 9./5.) + 32.
    }

    pub fn from_celsius<T: Into<f32>>(c: T) -> Temperature {
        let c: f32 = c.into();
        Temperature{0: (c - 32.0) * 5./9.}
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Pressure(f32);

impl Pressure {
    #[allow(non_snake_case)]
    pub fn inHg(&self) -> f32 {
        self.0*0.02952998057228486
    }

    #[allow(non_snake_case)]
    pub fn from_inHg<T: Into<f32>>(inHg: T) -> Pressure {
        let p: f32 = inHg.into();
        Pressure{0: p*33.863889532610884}
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Direction(u8); 

impl Direction {
    pub fn from_degrees(degrees: u16) -> Result<Direction, String> {

        let corrected_degrees = if degrees > 360  {
            return Err(format!("Degrees {degrees} is not a valid direction"));
        } else if degrees % 10 != 0 {
            return Err(format!("Degrees {degrees} is too precise. Precision will be rounded to nearest 10 degrees"));
        } else if degrees == 360 {
            0
        } else {
            (degrees / 10u16) as u8
        };

        Ok(Direction(corrected_degrees))
    }

    pub fn cardinal(&self) -> &'static str {
        match self.0 {
            0 => "N",
            1 => "N",
            2 => "NNE",
            3 => "NNE",
            4 => "NE",
            5 => "NE",
            6 => "ENE",
            7 => "ENE",
            8 => "E",
            9 => "E",
            10 => "E",
            11 => "ESE",
            12 => "ESE",
            13 => "SE",
            14 => "SE",
            15 => "SSE",
            16 => "SSE",
            17 => "S",
            18 => "S",
            19 => "S",
            20 => "SSW",
            21 => "SSW",
            22 => "SW",
            23 => "SW",
            24 => "WSW",
            25 => "WSW",
            26 => "W",
            27 => "W",
            28 => "W",
            29 => "WNW",
            30 => "WNW",
            31 => "NW",
            32 => "NW",
            33 => "NNW",
            34 => "NNW",
            35 => "N",
            _ => panic!("Direction struct contained {}, which is invalid.", self.0)
        }
    }

    pub fn degrees(&self) -> u16 {
        (self.0 as u16) * 10
    } 
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct WindSpeed(f32); // stored as m/s

impl WindSpeed {
    pub fn from_kts(kts: f32) -> WindSpeed{
        WindSpeed{0: kts*0.514444}
    }

    pub fn kts(&self) -> f32{
        self.0/0.514444
    }

    pub fn from_mps(meters_per_second: f32) -> WindSpeed {
        WindSpeed{0: meters_per_second}
    }

    pub fn mps(&self) -> f32 {
        self.0
    }

    pub fn from_mph(mph: f32) -> WindSpeed{
        WindSpeed{0: mph*0.44704}
    }

    pub fn mph(&self) -> f32 {
        self.0/0.44704
    }

    pub fn from_kph(kph: f32) -> WindSpeed{
        WindSpeed{0: kph*0.277778}
    }

    pub fn kph(&self) -> f32 {
        self.0/0.277778
    }

}


#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Wind {
    pub direction: Direction, // stored as degrees
    pub speed: WindSpeed,
}

impl Display for Wind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}° @ {} kts", self.direction.degrees(), self.speed.kts())
    }
}


#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum CloudLayerCoverage {
    Few,
    Scattered,
    Broken,
    Overcast
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


#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct CloudLayer {
    pub coverage: CloudLayerCoverage,
    pub height: u32, // given in feet
}

impl CloudLayer {   
    pub fn from_code(code: &str, height: u32) -> Result<Option<CloudLayer>, String> {
        let coverage_opt = match code {
            "SKC" => Ok(None),
            "CLR" => Ok(None),
            "FEW" => Ok(Some(CloudLayerCoverage::Few)),
            "SCT" => Ok(Some(CloudLayerCoverage::Scattered)),
            "BKN" => Ok(Some(CloudLayerCoverage::Broken)),
            "OVC" => Ok(Some(CloudLayerCoverage::Overcast)),
            _ => Err(format!("Unknown cloud cover string '{code}'")),
        }?;

        match coverage_opt {
            Some(coverage) => Ok(Some(CloudLayer {coverage, height})),
            None => Ok(None),
        }

    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum SkyCoverage {
    Clear,
    Cloudy(Vec<CloudLayer>),
}

// TODO: move this to wxer fetch.rs
impl SkyCoverage {
    pub fn from_vec_opt_str(cover: Vec<Option<String>>, level: Vec<Option<u32>>) -> Result<SkyCoverage, String> {
        
        if level.len() == 0 {return Ok(SkyCoverage::Clear)}

        let mut skyc = vec![];
        
        for l in cover.iter().zip(level.iter()) {
            match l {
                (Some(s), Some(l)) => {
                    let layer_option = CloudLayer::from_code(s, *l)?;

                    if let Some(layer) = layer_option {
                        skyc.push(layer)
                    }
                }

                (None, None) => {}
                _ => {return Err("Mismatched skycover and skylevel values".into())}
            }
        }

        Ok(SkyCoverage::Cloudy(skyc))
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Precip {
    pub unknown: f32,
    pub rain: f32,
    pub snow: f32,
}


#[derive(Clone, Serialize, Deserialize)]
pub struct StationEntry {
    pub indoor_temperature: Option<Temperature>,
    pub temperature_2m: Option<Temperature>,
    pub dewpoint_2m: Option<Temperature>,
    pub sea_level_pressure: Option<Pressure>,
    pub wind_10m: Option<Wind>, 
    pub skycover: Option<SkyCoverage>, 
    pub visibility: Option<f32>,
    pub precip_today: Option<Precip>,

    pub present_wx: Option<Vec<String>>,
    pub raw_metar: Option<String>, 
    pub raw_pressure: Option<Pressure>,
}
