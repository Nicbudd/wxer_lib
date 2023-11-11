use std::fmt::{Display, self};

use anyhow::{Result, bail};

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use derive_more::Display;

pub fn ignore_none<T, R, F: FnMut(T) -> R>(a: Option<T>, mut f: F) -> Option<R> {
    match a {
        None => None,
        Some(s) => {
            let r = f(s); 
            Some(r)
        }
    }
} 


#[derive(Clone, Copy, Serialize, Deserialize, Display)]
pub struct Temperature(f32); // stored as F

impl Temperature {
    pub fn celsius(&self) -> f32 {
        (self.0 * 9./5.) + 32.
    }

    pub fn fahrenheit(&self) -> f32 {
        self.0
    }

    pub fn from_celsius<T: Into<f32>>(c: T) -> Temperature {
        let c: f32 = c.into();
        Temperature{0: (c - 32.0) * 5./9.}
    }

    pub fn from_fahrenheit<T: Into<f32>>(f: T) -> Temperature {
        Temperature(f.into())
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Display)]
pub struct Pressure(f32); // stored as hPa

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

    #[allow(non_snake_case)]
    pub fn hPa(&self) -> f32 {
        self.0
    }

    #[allow(non_snake_case)]
    pub fn from_hPa<T: Into<f32>>(inHg: T) -> Pressure {
        let p: f32 = inHg.into();
        Pressure{0: p}
    }

}

#[derive(Clone, Copy, Serialize, Deserialize, Display)]
pub struct Direction(u16); 

impl Direction {
    fn sanitize_degrees(degrees: u16) -> Result<u16> {
        if degrees > 360 {
            bail!("Degrees provided ({degrees}) were not under 360.");
        } else if degrees % 10 != 0 {
            bail!("Degrees provided ({degrees}) were not divisible by 10.");
        } else if degrees == 360 {
            Ok(0)
        } else {
            Ok(degrees)
        }
    }

    pub fn from_degrees(degrees: u16) -> Result<Direction> {
        let corrected_degrees = Direction::sanitize_degrees(degrees)?;
        Ok(Direction(corrected_degrees))
    }

    pub fn cardinal(&self) -> &'static str {
        match self.0 {
            0 => "N",
            10 => "N",
            20 => "NNE",
            30 => "NNE",
            40 => "NE",
            50 => "NE",
            60 => "ENE",
            70 => "ENE",
            80 => "E",
            90 => "E",
            100 => "E",
            110 => "ESE",
            120 => "ESE",
            130 => "SE",
            140 => "SE",
            150 => "SSE",
            160 => "SSE",
            170 => "S",
            180 => "S",
            190 => "S",
            200 => "SSW",
            210 => "SSW",
            220 => "SW",
            230 => "SW",
            240 => "WSW",
            250 => "WSW",
            260 => "W",
            270 => "W",
            280 => "W",
            290 => "WNW",
            300 => "WNW",
            310 => "NW",
            320 => "NW",
            330 => "NNW",
            340 => "NNW",
            350 => "N",
            _ => panic!("Direction struct contained {}, which is invalid.", self.0)
        }
    }

    pub fn degrees(&self) -> u16 {
        self.0
    } 

    // pub fn add_degrees(&mut self, deg: i16) 

    //     let mut sum = self.0 as i16 + corrected_degrees;
    //     while sum < 0 {
    //         sum += 360;
    //     }
    //     self.0 = sum % 360;
    //     Ok(())
    // }  
}

#[derive(Clone, Copy, Serialize, Deserialize, Display)]
pub struct WindSpeed(f32); // stored as kts

impl WindSpeed {
    pub fn from_kts(kts: f32) -> WindSpeed{
        WindSpeed{0: kts}
    }

    pub fn kts(&self) -> f32{
        self.0
    }

    pub fn from_mps(mps: f32) -> WindSpeed {
        WindSpeed{0: mps*1.943844}
    }

    pub fn mps(&self) -> f32 {
        self.0/1.943844
    }

    pub fn from_mph(mph: f32) -> WindSpeed{
        WindSpeed{0: mph*0.868976}
    }

    pub fn mph(&self) -> f32 {
        self.0/0.868976
    }

    pub fn from_kph(kph: f32) -> WindSpeed{
        WindSpeed{0: kph*0.539957}
    }

    pub fn kph(&self) -> f32 {
        self.0/0.539957
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


#[derive(Serialize, Deserialize, Clone, Copy, Display)]
pub enum CloudLayerCoverage {
    #[display(fmt = "FEW")]
    Few,
    #[display(fmt = "SCT")]
    Scattered,
    #[display(fmt = "BKN")]
    Broken,
    #[display(fmt = "OVC")]
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
            Some(coverage) => Ok(Some(CloudLayer {coverage, height})),
            None => Ok(None),
        }
    }
}

impl fmt::Display for CloudLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {} ft", self.coverage.to_string(), self.height)
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum SkyCoverage {
    Clear,
    Cloudy(Vec<CloudLayer>),
}

impl fmt::Display for SkyCoverage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clear => write!(f, "CLR"),
            Self::Cloudy(v) => {
                write!(f, "{}", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
            }
        }
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
    pub date_time: DateTime<Utc>,
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

impl StationEntry {

    pub fn empty() -> StationEntry {
        StationEntry {
            date_time: DateTime::default(),
            indoor_temperature: None,
            temperature_2m: None,
            dewpoint_2m: None,
            sea_level_pressure: None,
            wind_10m: None,
            skycover: None,
            visibility: None,
            raw_metar: None,
            raw_pressure: None,
            precip_today: None,
            present_wx: None,
        }
    } 

    pub fn relative_humidity_2m(&self) -> Option<f32> { // in percentage
        if let (Some(temp_f), Some(dewp_f)) = (self.temperature_2m, self.dewpoint_2m) {
            let t = temp_f.celsius();
            let dp = dewp_f.celsius();
            let top_term = ((17.625 * dp)/(243.03 + dp)).exp();
            let bottom_term = ((17.625 * t)/(243.03 + t)).exp();
            Some(top_term / bottom_term * 100.)
        } else {
            None
        }
    }


    // fn merge_field<T: Clone>(base: &mut Option<T>, new: &Option<T>) {
    //     if base.is_none() {
    //         *base = new.clone();
    //     }
    // } 
    
    // fn merge_vec<T: Clone>(base: &mut Vec<T>, new: &Vec<T>) {
    //     if base.is_empty() {
    //         *base = new.clone();
    //     }
    // } 
    

    // pub fn merge_with(&mut self, e: &StationEntry) {
    //     merge_field(&mut self.indoor_temperature, &e.indoor_temperature);
    //     merge_field(&mut self.temperature_2m,     &e.temperature_2m);
    //     merge_field(&mut self.dewpoint_2m,        &e.dewpoint_2m);
    //     merge_field(&mut self.sea_level_pressure, &e.sea_level_pressure);
    //     merge_field(&mut self.wind_10m,           &e.wind_10m);
    //     merge_vec(  &mut self.skycover,           &e.skycover);
    //     merge_field(&mut self.visibility,         &e.visibility);
    //     merge_field(&mut self.precip_today,       &e.precip_today);
    //     merge_field(&mut self.present_wx,         &e.present_wx);
    //     merge_field(&mut self.raw_metar,          &e.raw_metar);
    //     merge_field(&mut self.raw_pressure,       &e.raw_pressure);
    // }

}



impl fmt::Debug for StationEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parameters: Vec<String> = vec![];

        if let Some(x) = self.indoor_temperature {
            parameters.push(format!("Inside Temp: {:3.1}", x)); 
        }

        if let Some(x) = self.temperature_2m {
            parameters.push(format!("Temp: {:3.1}", x)); 
        }

        if let Some(x) = self.dewpoint_2m {
            parameters.push(format!("Dew: {:3.1}", x)); 
        }

        if let Some(x) = self.sea_level_pressure {
            parameters.push(format!("MSLP: {:4.1}", x)); 
        }

        if let Some(x) = self.raw_pressure {
            parameters.push(format!("Sfc Pres: {:4.1}", x)); 
        }

        if let Some(w) = self.wind_10m {
            parameters.push(format!("Wind: {}", w)); 
        }

        if let Some(x) = self.visibility {
            parameters.push(format!("Vis: {:3.1}", x)); 
        }

        if let Some(s) = &self.skycover {
            parameters.push(s.to_string())
        }

        if let Some(x) = &self.raw_metar {
            parameters.push(format!("METAR: {}", x)); 
        }

        if let Some(x) = &self.present_wx {
            let mut s: String = String::new();
            
            if x.is_empty() {
                s += "Wx Codes: none";
            } else {
                s += "Wx Codes:";

                for a in x {
                    parameters.push(a.clone()); 
                }
            }
        }


        let full_string = parameters.join(", ");

        write!(f, "{}", full_string)
    }
}
