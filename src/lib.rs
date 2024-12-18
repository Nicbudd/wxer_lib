use std::f32::consts::PI;
use std::fmt::{Display, self};
use std::collections::HashMap;

use anyhow::{Result, bail};

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use derive_more::Display;
use regex::Regex;

pub mod fetch;
// use fetch::*;

pub mod formulae;
use formulae::*;

pub mod db;
// pub use db::*;

// STRUCTS ---------------------------------------------------------------------



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Station {
    pub name: String,
    pub altitude: f32,
    pub coords: (f32, f32),
}



// WXENTRY

#[derive(Clone, Serialize, Deserialize)]
pub struct WxEntry {
    pub date_time: DateTime<Utc>,
    pub station: Station,
    
    pub layers: HashMap<Layer, WxEntryLayer>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cape: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skycover: Option<SkyCoverage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wx_codes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wx: Option<Wx>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_metar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precip_today: Option<Precip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precip: Option<Precip>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precip_probability: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub altimeter: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_slp: Option<f32>,
}

impl WxEntry {

    pub fn empty(station: &Station) -> WxEntry {
        WxEntry {
            date_time: DateTime::default(),
            station: station.clone(),

            layers: HashMap::new(),
            
            cape: None,
            skycover: None,
            precip_today: None,
            precip: None,
            precip_probability: None,
            wx_codes: None,
            wx: None,
            raw_metar: None,
            altimeter: None,
            
            best_slp: None,
        }
    } 

    pub fn latitude(&self) -> f32 {
        return self.station.coords.0;
    }

    pub fn best_slp(&self) -> Option<f32> {
        let option_1 = {self.layers.get(&SeaLevel).map(|x| x.pressure).flatten()};
        let option_2 = {self.layers.get(&NearSurface).map(|x| x.slp(self.latitude())).flatten()};
        let option_3 = {self.layers.get(&Indoor).map(|x| x.slp(self.latitude())).flatten()};
        let option_4 = {self.altimeter_to_slp()};

        option_1.or(option_2).or(option_3).or(option_4)
    }

    pub fn altimeter_to_station(&self) -> Option<f32> {
        Some(altimeter_to_station(self.altimeter?, self.station.altitude))
    }

    pub fn altimeter_to_slp(&self) -> Option<f32> {
        let surface = self.layers.get(&NearSurface)?;
        Some(altimeter_to_slp(self.altimeter?, self.station.altitude, surface.temperature?))
    }


    pub fn sealevel(&self) -> Option<&WxEntryLayer> {
        self.layers.get(&SeaLevel)
    }

    pub fn surface(&self) -> Option<&WxEntryLayer> {
        self.layers.get(&NearSurface)
    }

    pub fn indoor(&self) -> Option<&WxEntryLayer> {
        self.layers.get(&Indoor)
    }

    pub fn wx_from_codes(&self) -> Option<Wx> {
        let mut wx = Wx::none();
        let codes = self.wx_codes.clone()?; // stupid, why do I have to clone here
        for code in codes {
            wx = wx.combine(Wx::parse_code(&code));
        }
        Some(wx)
    }

    pub fn fill_in_calculated_values(&mut self) {
        let lat = self.latitude();
        let alt = self.altimeter;
        for e in self.layers.iter_mut() {
            e.1.fill_in_calculated_values(lat, alt);
        }

        self.wx_from_codes();
        self.best_slp = self.best_slp();
    }
}



// #[derive(Clone, Serialize, Deserialize)]
// pub struct WxEntryCalculated {
//     #[serde(skip_serializing_if = "Option::is_none")]
//     

//     pub layers: HashMap<Layer, WxEntryLayerCalculated>,
// }


impl fmt::Debug for WxEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        let mut parameters: Vec<String> = vec![];

        parameters.push(self.date_time.to_string());
        parameters.push(format!("{:?}", self.station));

        if let Some(s) = &self.cape {
            parameters.push(format!("CAPE: {s:.0}"))
        }

        if let Some(s) = &self.skycover {
            parameters.push(s.to_string())
        }

        if let Some(s) = &self.precip_probability {
            parameters.push(format!("Precip Prob: {}", s.to_string()))
        }

        if let Some(s) = &self.precip_today {
            parameters.push(s.to_string())
        }

        if let Some(s) = &self.precip {
            parameters.push(s.to_string())
        }

        if let Some(x) = &self.raw_metar {
            parameters.push(format!("METAR: {}", x)); 
        }



        if let Some(x) = &self.wx_codes {
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

        let layer_string = self.layers
                            .iter()
                            .map(|(_, x)| x.to_string())
                            .collect::<Vec<String>>()
                            .join(", ");

        write!(f, "{}, {}", parameters.join(", "), layer_string)


    }
}


// WXENTRYLAYER

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct WxEntryLayer {
    pub layer: Layer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_agl: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height_msl: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dewpoint: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressure: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_direction: Option<Direction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_speed: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<f32>,

    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub wind: Option<Wind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relative_humidity: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slp: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wind_chill: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heat_index: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apparent_temp: Option<f32>,
        
    // only include if given air is below the lcl
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theta_e: Option<f32>,
}

// #[derive(Clone, Serialize, Deserialize)]
// pub struct WxEntryLayerCalculated {

// }


impl WxEntryLayer {
    pub fn empty(layer: Layer) -> WxEntryLayer {
        WxEntryLayer {
            layer,
            height_agl: None,
            height_msl: None,
            temperature: None,
            dewpoint: None,
            pressure: None,
            wind_direction: None,
            wind_speed: None,
            visibility: None,

            // wind: None,
            relative_humidity: None,
            slp: None,
            wind_chill: None,
            heat_index: None,
            apparent_temp: None,
            theta_e: None,
        }
    }

    pub fn wind(&self) -> Option<Wind> {
        if let (Some(direction), Some(speed)) = (self.wind_direction, self.wind_speed) {
            Some(Wind {
                direction,
                speed
            })
        } else {
            None
        } 
    }

    pub fn relative_humidity(&self) -> Option<f32> { // in percentage
        if let (Some(temp_f), Some(dewp_f)) = (self.temperature, self.dewpoint) {
            let t = f_to_c(temp_f);
            let dp = f_to_c(dewp_f);
            let top_term = ((17.625 * dp)/(243.03 + dp)).exp();
            let bottom_term = ((17.625 * t)/(243.03 + t)).exp();
            Some(top_term / bottom_term * 100.)
        } else {
            None
        }
    }

    pub fn slp(&self, latitude: f32) -> Option<f32> {
        if let (Some(p), Some(t), Some(h)) = (self.pressure, self.temperature, self.height_msl) {
            // http://www.wind101.net/sea-level-pressure-advanced/sea-level-pressure-advanced.html
            let phi =  latitude * PI / 180.0;
            let b = 1013.25; //(average baro pressure of a column)
            let k_upper =  18400.; // meters apparently
            let alpha = 0.0037; // coefficient of thermal expansion of air
            let k_lower = 0.0026; // based on figure of earth
            let r = 6367324.; // radius of earth
            
            let lapse_rate = 0.005; // 0.5C/100m

            let column_temp = f_to_c(t) + (lapse_rate*h)/2.; // take the average of the temperature
            // dbg!(&column_temp);
            let e = 10f32.powf(7.5*column_temp / (237.3+column_temp)) * 6.1078;
            // dbg!(&e);

            let term1 = 1. + (alpha * column_temp); // correction for column temp
            // dbg!(&term1);
            let term2 = 1. / (1. - (0.378 * (e/b))); // correction for humidity
            // dbg!(&term2);
            let term3 = 1. / (1. - (k_lower * (2.*phi).cos())); // correction for obliquity of earth
            // dbg!(&term3);
            let term4 = 1. + (h/r); // correction for gravity
            // dbg!(&term4);

            let correction = h / (k_upper*term1*term2*term3*term4);
            // dbg!(&h);

            let mslp = 10f32.powf(p.log10() + correction);

            Some(mslp)

        } else {
            None
        }
    }

    // None - Incomplete Data
    // Some(true) - wind chill is within valid temp & wind range
    // Some(false) - wind chill is outside valid temp and wind range
    pub fn wind_chill_valid(&self) -> Option<bool> {
        if let Some(t) = self.temperature {
            if t < 50. {
                if let Some(w) = self.wind_speed {
                    Some(kts_to_mph(w) > 3.)
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
        if let (Some(w), Some(t)) = (self.wind_speed, self.temperature) {
            let mph = kts_to_mph(w);

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
        if let Some(t) = self.temperature {
            if t > 80. {
                if let Some(rh) = self.relative_humidity() {
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
        if let (Some(t), Some(rh)) = (self.temperature, self.relative_humidity()) {
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

        if let Some(_) = self.temperature {
            match (self.heat_index_valid(), self.wind_chill_valid()) {
                (Some(true), _) => self.heat_index(), // if the heat index is valid, use it
                (_, Some(true)) => self.wind_chill(), // if the wind chill is valid, use it
                (None, _) | (_, None) => None, // if neither are valid and we're missing data, then we can't provide a valid index
                (Some(false), Some(false)) => self.temperature, // if we're outside the range of both, then we can just use temp_2m
            }

        } else {
            None
        }
    }

    pub fn theta_e(&self, altimeter: Option<f32>, altitude: Option<f32>) -> Option<f32> {
        if let (Some(temp_f), Some(dewp_f)) = (self.temperature, self.dewpoint) {
            
            let pressure;
            if let Some(p) = self.pressure {
                pressure = p;
            } else if let Some(alt_pres) =  altimeter {
                pressure = formulae::altimeter_to_station(alt_pres, altitude?)
            } else {
                return None
            }

            return Some(formulae::theta_e(f_to_k(temp_f), f_to_k(dewp_f), pressure));
        }

        return None;
    }

    pub fn fill_in_calculated_values(&mut self, latitude: f32, altimeter: Option<f32>) {
        // self.wind = self.wind();
        self.relative_humidity = self.relative_humidity();
        self.slp = self.slp(latitude);
        self.wind_chill = self.wind_chill();
        self.heat_index = self.heat_index();
        self.apparent_temp = self.apparent_temp();
        self.theta_e = self.theta_e( altimeter, self.height_msl);
    }

}


impl fmt::Display for WxEntryLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parameters: Vec<String> = vec![];

        // parameters.push(format!("{}", self.date_time.format("%c"))); 

        parameters.push(format!("Level: {}", self.layer)); 

        if let Some(x) = self.height_agl {
            parameters.push(format!("Height AGL: {:.0}", x));
        }

        if let Some(x) = self.height_msl {
            parameters.push(format!("Height MSL: {:.0}", x));
        }

        if let Some(x) = self.temperature {
            parameters.push(format!("Temp: {:3.1}", x)); 
        }

        if let Some(x) = self.dewpoint {
            parameters.push(format!("Dew: {:3.1}", x)); 
        }

        if let Some(x) = self.pressure {
            parameters.push(format!("Pres: {:4.1}", x)); 
        }

        if let Some(w) = self.wind_speed {
            parameters.push(format!("Wind Speed: {}", w)); 
        }

        if let Some(w) = self.wind_direction {
            parameters.push(format!("Wind Direction: {}", w)); 
        }

        if let Some(x) = self.visibility {
            parameters.push(format!("Vis: {:3.1}", x)); 
        }


        let full_string = parameters.join(", ");

        write!(f, "{}", full_string)
    }
}

// LAYER

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Layer {
    Indoor,
    NearSurface,
    SeaLevel,
    AGL(u64),
    MSL(u64),
    MBAR(u64),
}

use Layer::*;

impl Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Indoor => write!(f, "Indoor"),
            NearSurface => write!(f, "Near Surface"),
            SeaLevel => write!(f, "Sea Level"),
            AGL(h) => write!(f, "{h} ft AGL"),
            MSL(h) => write!(f, "{h} ft MSL"),
            MBAR(h) => write!(f, "{h} mb"),
        }
    }
}


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

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Precip {
    pub unknown: f32,
    pub rain: f32,
    pub snow: f32,
}

impl Display for Precip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rain: {}, Snow: {}, Unknown: {}", self.rain, self.snow, self.unknown)
    }
}




// HELPER FUNCTIONS ------------------------------------------------------------

pub fn ignore_none<T, R, F: FnMut(T) -> R>(a: Option<T>, mut f: F) -> Option<R> {
    match a {
        None => None,
        Some(s) => {
            let r = f(s); 
            Some(r)
        }
    }
} 



#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Wx { 
    pub blowing: bool,
    pub freezing: bool,
    pub showers: bool,
    pub squalls: bool,
    pub thunderstorm: bool,
    pub fog: bool,
    pub smoke: bool,
    
    pub visibility_inhibitor: bool,
    
    #[serde(skip_serializing_if = "Intensity::is_none")]
    pub rain: Intensity,
    #[serde(skip_serializing_if = "Intensity::is_none")]
    pub snow: Intensity,
    #[serde(skip_serializing_if = "Intensity::is_none")]
    pub falling_ice: Intensity,    
    #[serde(skip_serializing_if = "Intensity::is_none")]
    pub dust: Intensity,
    #[serde(skip_serializing_if = "Intensity::is_none")]
    pub sand: Intensity,
    #[serde(skip_serializing_if = "Intensity::is_none")]
    pub funnel_cloud: Intensity, // light: FC, heavy: Tornado
    #[serde(skip_serializing_if = "Intensity::is_none")]
    pub unknown: Intensity, // light: FC, heavy: Tornado
}

impl Wx {
    pub fn none() -> Wx {
        use Intensity::None;
        Wx {
            blowing: false, freezing: false, showers: false, squalls: false, 
            thunderstorm: false, visibility_inhibitor: false, fog: false,
            smoke: false,
            unknown: None, rain: None, snow: None, falling_ice: None, 
            dust: None, sand: None, funnel_cloud: None,
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
        wx.blowing = matches.contains(&"BL") || matches.contains(&"SS") || matches.contains(&"PO") || matches.contains(&"DS");
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

        if matches.contains(&"SN") || matches.contains(&"GS") || 
           matches.contains(&"IC") || matches.contains(&"SG") {
            wx.snow = general_intensity;
        }

        if matches.contains(&"FC") {
            wx.funnel_cloud = general_intensity;
        }

        // for now we'll intentionally ignore BC,DR,MI,PR,PY,NSW
        wx
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
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
        return self == &Self::None
    }
    pub fn most_intense(self, other: Intensity) -> Intensity {
        if self > other {
            self
        } else {
            other
        }
    }
}

// #[derive(Debug, Clone, Serialize)]
// pub struct WxCode {
//     pub code: String,
//     pub partial_wx: Wx
// }

// impl WxCode {

// }





#[cfg(test)]
mod tests {
    use crate::WxEntryLayer;
    use crate::Layer::*;

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
        let mut e = WxEntryLayer::empty(NearSurface);

        e.temperature = Some(51.);
        assert_eq!(e.apparent_temp(), Some(51.));

        e.temperature = Some(49.);
        assert_eq!(e.apparent_temp(), None);

        e.wind_speed = Some(2.);
        assert_eq!(e.apparent_temp(), Some(49.));

        e.temperature = Some(79.);
        assert_eq!(e.apparent_temp(), Some(79.));

        e.temperature = Some(81.);
        assert_eq!(e.apparent_temp(), None);

        e.dewpoint = Some(54.);
        assert_eq!(e.apparent_temp(), Some(81.));


        // heat index tests

        e.temperature = Some(81.);
        e.dewpoint = Some(65.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 82.8));


        e.temperature = Some(100.);
        e.dewpoint = Some(75.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 113.7));

        e.temperature = Some(110.);
        e.dewpoint = Some(85.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 146.1));


        // wind chill tests

        e.temperature = Some(32.);
        e.wind_speed = Some(10.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 23.0));

        e.temperature = Some(49.);
        e.wind_speed = Some(3.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 48.1));

        e.temperature = Some(49.);
        e.wind_speed = Some(40.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, 38.9));

        e.temperature = Some(-20.);
        e.wind_speed = Some(3.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, -30.7));

        e.temperature = Some(-20.);
        e.wind_speed = Some(40.);
        let apparent_temp = e.apparent_temp().unwrap_or(0.0);
        assert!(float_within_one_decimal(apparent_temp, -58.4));

    }
}

