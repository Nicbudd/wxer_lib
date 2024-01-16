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


pub fn c_to_f<T: Into<f32>>(f: T) -> f32 {
    let f = f.into();
    (f * 9./5.) + 32.
}

pub fn f_to_c<T: Into<f32>>(c: T) -> f32 {
    let c: f32 = c.into();
    (c - 32.0) * 5./9.
}

// #[allow(non_snake_case)]
// pub fn inHg(&self) -> f32 {
//     self.0*0.02952998057228486
// }

#[derive(Clone, Copy, Serialize, Deserialize, Display)]
pub struct Direction(u16); 

impl Direction {
    fn sanitize_degrees(degrees: u16) -> Result<u16> {
        let degrees = if degrees > 360 {
            bail!("Degrees provided ({degrees}) were not under 360.");
        } else if degrees % 10 != 0 {
            ((degrees + 5) / 10) * 10 // round to nearest 10
        } else {
            degrees
        };

        Ok(degrees % 360)
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
            _ => unreachable!("Direction struct contained {}, which is invalid.", self.0)
        }
    }

    pub fn degrees(&self) -> u16 {
        self.0
    } 
}

pub fn kts_to_mph(f: f32) -> f32 {
    f/0.868976
}

pub fn kts_to_kph(f: f32) -> f32 {
    f/0.539957
}


#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Wind {
    pub direction: Direction, // stored as degrees
    pub speed: f32,
}

impl Display for Wind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}°@{} kts", self.direction.degrees(), self.speed)
    }
}


#[derive(Serialize, Deserialize, Clone, Copy, Display, PartialEq)]
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
        write!(f, "{}@{} ft", self.coverage.to_string(), self.height)
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

#[derive(Debug, Clone)]
pub struct Station {
    pub name: String,
    pub altitude: f32,
    pub coords: (f32, f32),
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
    pub indoor_temperature: Option<f32>,
    pub temperature_2m: Option<f32>,
    pub dewpoint_2m: Option<f32>,
    pub sea_level_pressure: Option<f32>,
    pub wind_10m: Option<Wind>, 
    pub skycover: Option<SkyCoverage>, 
    pub visibility: Option<f32>,
    pub precip_today: Option<Precip>,
    pub present_wx: Option<Vec<String>>,
    pub raw_metar: Option<String>, 
    pub raw_pressure: Option<f32>,
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
            let t = f_to_c(temp_f);
            let dp = f_to_c(dewp_f);
            let top_term = ((17.625 * dp)/(243.03 + dp)).exp();
            let bottom_term = ((17.625 * t)/(243.03 + t)).exp();
            Some(top_term / bottom_term * 100.)
        } else {
            None
        }
    }

    pub fn slp(&self, station: &Station) -> Option<f32> {
        if let (Some(p), Some(t)) = (self.raw_pressure, self.temperature_2m) {
            // http://www.wind101.net/sea-level-pressure-advanced/sea-level-pressure-advanced.html
            let h = station.altitude;
            let latitude = station.coords.0;
            let b = 1013.25; //(average baro pressure of a column)
            let k_upper =  18400.; // meters apparently
            let alpha = 0.0037; // coefficient of thermal expansion of air
            let k_lower = 0.0026; // based on figure of earth
            let r = 6367324.; // radius of earth
            
            let lapse_rate = if h < 100. {
                0. // assume the boundary layer is about 100 meters
            } else {
                0.05
            };

            let column_temp = t + (lapse_rate*h)/2.; // take the average of the temperature
            let e = 10f32.powf(7.5*column_temp / (237.3+column_temp)) * 6.1078;

            let term1 = 1. + (alpha * column_temp); // correction for column temp
            let term2 = 1. / (1. - (0.378 * (e/b))); // correction for humidity
            let term3 = 1. / (1. - (k_lower * (2.*latitude).cos())); // correction for obliquity of earth
            let term4 = 1. + (h/r); // correction for gravity

            let correction = h / (k_upper*term1*term2*term3*term4);

            let mslp = p * 10f32.powf(10f32.log10() - correction);

            Some(mslp)

        } else {
            self.sea_level_pressure
        }
    }


    // None - Incomplete Data
    // Some(true) - wind chill is within valid temp & wind range
    // Some(false) - wind chill is outside valid temp and wind range
    pub fn wind_chill_valid(&self) -> Option<bool> {
        if let Some(t) = self.temperature_2m {
            if t < 50. {
                if let Some(w) = self.wind_10m {
                    Some(kts_to_mph(w.speed) > 3.)
                } else {
                    None
                }
            } else {
                Some(false)
            }
        } else {
            None
        }
    }

    pub fn wind_chill(&self) -> Option<f32> {
        if let (Some(w), Some(t)) = (self.wind_10m, self.temperature_2m) {
            let mph = kts_to_mph(w.speed);

            if self.wind_chill_valid() == Some(true) {
                let v_016 = mph.powf(0.16);
                Some(35.74 + 0.6215*t - 35.75*v_016 + 0.4275*t*v_016)
            } else {
                None
            }

        } else {
            None
        }
    }


    // None - Incomplete Data
    // Some(true) - heat index is within valid temp & humidity range
    // Some(false) - heat index is outside valid temp & humidity range
    pub fn heat_index_valid(&self) -> Option<bool> {
        if let Some(t) = self.temperature_2m {
            if t > 80. {
                if let Some(rh) = self.relative_humidity_2m() {
                    Some(rh > 40.)
                } else {
                    None
                }
            } else {
                Some(false)
            }
        } else {
            None
        }
    }

    // from Wikipedia: https://en.wikipedia.org/wiki/Heat_index
    pub fn heat_index(&self) -> Option<f32> {
        if let (Some(t), Some(rh)) = (self.temperature_2m, self.relative_humidity_2m()) {
            if self.heat_index_valid() == Some(true) {
                const C: [f32; 10] = [0.0, -42.379, 2.04901523, 10.14333127, -0.22475541, -0.00683783, -0.05481717, 0.00122874, 0.00085282, -0.00000199];
                Some((C[1]) + (C[2]*t) + (C[3]*rh) + (C[4]*t*rh) + (C[5]*t*t) + (C[6]*rh*rh) + (C[7]*t*t*rh) + (C[8]*t*rh*rh) + (C[9]*t*t*rh*rh))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn apparent_temp(&self) -> Option<f32> {

        // dbg!(self.heat_index_valid(), self.wind_chill_valid());

        if let Some(_) = self.temperature_2m {
            match (self.heat_index_valid(), self.wind_chill_valid()) {
                (Some(true), _) => self.heat_index(), // if the heat index is valid, use it
                (_, Some(true)) => self.wind_chill(), // if the wind chill is valid, use it
                (None, _) | (_, None) => None, // if neither are valid and we're missing data, then we can't provide a valid index
                (Some(false), Some(false)) => self.temperature_2m, // if we're outside the range of both, then we can just use temp_2m
            }

        } else {
            None
        }
    }

}



impl fmt::Debug for StationEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parameters: Vec<String> = vec![];

        parameters.push(format!("{}", self.date_time.format("%c"))); 
        

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

#[cfg(test)]
mod tests {
    use crate::{StationEntry, Wind};

    fn float_within_one_decimal(val: f32, cmp: f32) -> bool {
        if val < (cmp + 0.1) && val > (cmp - 0.1) {
            true
        } else {
            println!("{val}");
            false
        }
    }

    #[test]
    fn test_apparent_temp() {
        let mut e = StationEntry::empty();

        e.temperature_2m = Some(51.);
        assert_eq!(e.apparent_temp(), Some(51.));

        e.temperature_2m = Some(49.);
        assert_eq!(e.apparent_temp(), None);

        e.wind_10m = Some(Wind {speed: 2., direction: crate::Direction::from_degrees(0).unwrap()});
        assert_eq!(e.apparent_temp(), Some(49.));

        e.temperature_2m = Some(79.);
        assert_eq!(e.apparent_temp(), Some(79.));

        e.temperature_2m = Some(81.);
        assert_eq!(e.apparent_temp(), None);

        e.dewpoint_2m = Some(54.);
        assert_eq!(e.apparent_temp(), Some(81.));


        // heat index tests

        e.temperature_2m = Some(81.);
        e.dewpoint_2m = Some(65.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 82.8));


        e.temperature_2m = Some(100.);
        e.dewpoint_2m = Some(75.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 113.7));

        e.temperature_2m = Some(110.);
        e.dewpoint_2m = Some(85.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 146.1));


        // wind chill tests

        e.temperature_2m = Some(32.);
        e.wind_10m = Some(Wind {speed: 10., direction: crate::Direction::from_degrees(0).unwrap()});
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 23.0));

        e.temperature_2m = Some(49.);
        e.wind_10m = Some(Wind {speed: 3., direction: crate::Direction::from_degrees(0).unwrap()});
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 48.1));

        e.temperature_2m = Some(49.);
        e.wind_10m = Some(Wind {speed: 40., direction: crate::Direction::from_degrees(0).unwrap()});
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 38.9));

        e.temperature_2m = Some(-20.);
        e.wind_10m = Some(Wind {speed: 3., direction: crate::Direction::from_degrees(0).unwrap()});
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, -30.7));

        e.temperature_2m = Some(-20.);
        e.wind_10m = Some(Wind {speed: 40., direction: crate::Direction::from_degrees(0).unwrap()});
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, -58.4));

    }
}