use std::{any::Any, collections::{HashMap, HashSet}, f32::consts::PI, fmt::{self, Display}, hash::Hash};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use derive_more::Display;

use crate::units::*;
use crate::formulae;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Station {
    pub name: String,
    pub altitude: Altitude,
    pub coords: (f32, f32),
}



pub trait WxEntry<'a, L: WxEntryLayer + 'a> where Self: Sized + fmt::Debug {

    // REQUIRED FIELDS ---------------------------------------------------------
    fn date_time(&self) -> DateTime<Utc>;
    fn station(&self) -> Station;
    fn layer(&'a self, layer: Layer) -> Option<L>;
    fn layers(&self) -> Vec<Layer>;
    // fn new(station: &Station) -> Self;

    // OPTIONAL FIELDS ---------------------------------------------------------
    fn skycover(&self) -> Option<SkyCoverage> {None}
    fn wx_codes(&self) -> Option<Vec<String>> {None}
    fn raw_metar(&self) -> Option<String> {None}
    fn precip_today(&self) -> Option<Precip> {None}
    fn precip(&self) -> Option<Precip> {None}
    fn altimeter(&self) -> Option<Pressure> {None}
    fn cape(&self) -> Option<SpecEnergy> {None} 

    // CALCULATED FIELDS -------------------------------------------------------

    fn wx(&self) -> Option<Wx> {
        let mut wx = Wx::none();
        let codes = self.wx_codes().clone()?; // stupid, why do I have to clone here
        for code in codes {
            wx = wx.combine(Wx::parse_code(&code));
        }
        Some(wx)
    }

    fn latitude(&self) -> f32 {
        return self.station().coords.0;
    }

    fn best_slp(&'a self) -> Option<Pressure> {
        let option_1 = {self.layer(SeaLevel).map(|x| x.pressure()).flatten()};
        let option_2 = {self.layer(NearSurface).map(|x| x.slp(self.latitude())).flatten()};
        let option_3 = {self.layer(Indoor).map(|x| x.slp(self.latitude())).flatten()};
        let option_4 = {self.mslp_from_altimeter()};

        option_1.or(option_2).or(option_3).or(option_4)
    }


    // ACCESSOR FIELDS ---------------------------------------------------------

    fn sealevel(&'a self) -> Option<L> {
        self.layer(SeaLevel)
    }

    fn surface(&'a self) -> Option<L> {
        self.layer(NearSurface)
    }

    fn indoor(&'a self) -> Option<L> {
        self.layer(Indoor)
    }

    fn to_struct(&'a self) -> Result<WxEntryStruct> {
        let mut layers = HashMap::new();
        
        for layer in self.layers() {
            let layer = self.layer(layer).context("Layer in layers() was not contained in layer(layer).")?;
            
            let l = WxEntryLayerStruct {
                layer: layer.layer(),
                station: layer.station(),
                temperature: layer.temperature(),
                pressure: layer.pressure(),
                dewpoint: layer.dewpoint(),
                visibility: layer.visibility(),
                wind: layer.wind(),
            };

            layers.insert(layer.layer(), l);
        }

        Ok(WxEntryStruct {
            altimeter: self.altimeter(),
            cape: self.cape(),
            date_time: self.date_time(),
            station: self.station(),
            layers,
            skycover: self.skycover(),
            wx_codes: self.wx_codes(),
            raw_metar: self.raw_metar(),
            precip_today: self.precip_today(),
            precip: self.precip()
        })
    }


    // FOR USE BY IMPLEMENTORS -------------------------------------------------

    fn station_pressure_from_altimeter(&self) -> Option<Pressure> {
        Some(formulae::altimeter_to_station(self.altimeter()?, self.station().altitude))
    }

    fn mslp_from_altimeter(&'a self) -> Option<Pressure> {
        let surface = self.layer(NearSurface)?;
        Some(formulae::altimeter_to_slp(self.altimeter()?, self.station().altitude, surface.temperature()?))
    }
}



pub trait WxEntryLayer {
    fn layer(&self) -> Layer;
    fn station(&self) -> Station;


    // OPTIONAL FIELDS ---------------------------------------------------------

    fn temperature(&self) -> Option<Temperature> {None}
    fn pressure(&self) -> Option<Pressure> {None}
    fn visibility(&self) -> Option<Distance> {None}

    // QUASI-CALCULATED FIELDS -------------------------------------------------
    // completing one of these fields will complete the others

    fn dewpoint(&self) -> Option<Temperature> {self.dewpoint_from_rh()}
    fn relative_humidity(&self) -> Option<Fraction> {self.rh_from_dewpoint()}

    fn wind_speed(&self) -> Option<Speed> {self.wind_speed_from_wind()}
    fn wind_direction(&self) -> Option<Direction> {self.wind_direction_from_wind()}
    fn wind(&self) -> Option<Wind> {self.wind_from_speed_and_direction()}


    // CALCULATED FIELDS -------------------------------------------------------

    fn height_agl(&self) -> Altitude {
        self.layer().height_agl(self.height_msl())
    }

    fn height_msl(&self) -> Altitude {
        self.height_agl() + self.station().altitude
    }

    fn slp(&self, latitude: f32) -> Option<Pressure> {
        let p = self.pressure()?.value_in(Mbar);
        let t = self.temperature()?.value_in(Celsius);
        let h = self.height_msl().value_in(Meter);
        
        // http://www.wind101.net/sea-level-pressure-advanced/sea-level-pressure-advanced.html
        let phi =  latitude * PI / 180.0;
        let b = 1013.25; //(average baro pressure of a column)
        let k_upper =  18400.; // meters apparently
        let alpha = 0.0037; // coefficient of thermal expansion of air
        let k_lower = 0.0026; // based on figure of earth
        let r = 6367324.; // radius of earth
        
        let lapse_rate = 0.005; // 0.5C/100m

        let column_temp = t + (lapse_rate*h)/2.; // take the average of the temperature
        let e = 10f32.powf(7.5*column_temp / (237.3+column_temp)) * 6.1078;

        let term1 = 1. + (alpha * column_temp); // correction for column temp
        let term2 = 1. / (1. - (0.378 * (e/b))); // correction for humidity
        let term3 = 1. / (1. - (k_lower * (2.*phi).cos())); // correction for obliquity of earth
        let term4 = 1. + (h/r); // correction for gravity

        let correction = h / (k_upper*term1*term2*term3*term4);

        let mslp = 10f32.powf(p.log10() + correction);

        Some(Pressure::new(mslp, Mbar))
    }

    // None - Incomplete Data
    // Some(true) - wind chill is within valid temp & wind range
    // Some(false) - wind chill is outside valid temp and wind range
    fn wind_chill_valid(&self) -> Option<bool> {
        let t = self.temperature()?.value_in(Fahrenheit);
        if t < 50. {
            let mph = self.wind_speed()?.value_in(Mph);
            Some(mph > 3.)
        } else {
            Some(false)
        }
    }

    fn wind_chill(&self) -> Option<Temperature> {
        let mph = self.wind_speed()?.value_in(Mph);
        let t = self.temperature()?.value_in(Fahrenheit);

        if self.wind_chill_valid() == Some(true) {
            let v_016 = mph.powf(0.16);
            let wc_f = 35.74 + 0.6215*t - 35.75*v_016 + 0.4275*t*v_016;
            Some(Temperature::new(wc_f, Fahrenheit))
        } else {
            None
        }

    }

    // None - Incomplete Data
    // Some(true) - heat index is within valid temp & humidity range
    // Some(false) - heat index is outside valid temp & humidity range
    fn heat_index_valid(&self) -> Option<bool> {
        let t = self.temperature()?.value_in(Fahrenheit);

        if t > 80. {
            let rh = self.relative_humidity()?.value_in(Percent);
            Some(rh > 40.)
        } else {
            Some(false)
        }
    }

    // from Wikipedia: https://en.wikipedia.org/wiki/Heat_index
    fn heat_index(&self) -> Option<Temperature> {
        let t = self.temperature()?.value_in(Fahrenheit);
        let rh = self.relative_humidity()?.value_in(Percent);

        if self.heat_index_valid() == Some(true) {
            const C: [f32; 10] = [0.0, -42.379, 2.04901523, 10.14333127, -0.22475541, -0.00683783, -0.05481717, 0.00122874, 0.00085282, -0.00000199];
            let hi_f = (C[1]) + (C[2]*t) + (C[3]*rh) + (C[4]*t*rh) + (C[5]*t*t) + (C[6]*rh*rh) + (C[7]*t*t*rh) + (C[8]*t*rh*rh) + (C[9]*t*t*rh*rh);
            Some(Temperature::new(hi_f, Fahrenheit))
        } else {
            None
        }

    }

    fn apparent_temp(&self) -> Option<Temperature> {
        // dbg!(self.heat_index_valid(), self.wind_chill_valid());
        let _ = self.temperature()?;

        match (self.heat_index_valid(), self.wind_chill_valid()) {
            (Some(true), _) => self.heat_index(), // if the heat index is valid, use it
            (_, Some(true)) => self.wind_chill(), // if the wind chill is valid, use it
            (Some(false), Some(false)) => self.temperature(), // if we're outside the range of both, then we can just use temp_2m
            (None, _) | (_, None) => None, // if neither are valid and we're missing data, then we can't provide a valid index
        }

    }

    fn theta_e(&self, altimeter: Option<Pressure>) -> Option<Temperature> {   

        let pressure;
        if let Some(p) = self.pressure() {
            pressure = p;
        } else if let Some(alt_pres) =  altimeter {
            pressure = formulae::altimeter_to_station(alt_pres, self.height_msl())
        } else {
            return None
        }

        return Some(formulae::theta_e(self.temperature()?, self.dewpoint()?, pressure));
    }



    // QUASI-CALCULATED IMPLEMENTATIONS ----------------------------------------

    fn dewpoint_from_rh(&self) -> Option<Temperature> {
        Some(formulae::rh_to_dewpoint(self.temperature()?, self.relative_humidity()?))
    }

    fn rh_from_dewpoint(&self) -> Option<Fraction> { // in percentage
        let t = self.temperature()?.value_in(Celsius);
        let td = self.dewpoint()?.value_in(Celsius);
        let top_term = ((17.625 * td)/(243.03 + td)).exp();
        let bottom_term = ((17.625 * t)/(243.03 + t)).exp();
        Some(Fraction::new(top_term / bottom_term, Decimal))
    }

    fn wind_from_speed_and_direction(&self) -> Option<Wind> {
        Some(Wind {direction: self.wind_direction()?, speed: self.wind_speed()?})
    }

    fn wind_speed_from_wind(&self) -> Option<Speed> {
        Some(self.wind()?.speed)
    }

    fn wind_direction_from_wind(&self) -> Option<Direction> {
        Some(self.wind()?.direction)
    }
}



// LAYER

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
pub enum Layer {
    All,
    Indoor,
    NearSurface,
    SeaLevel,
    AGL(u64), // in m
    MSL(u64), // in m
    MBAR(u64, u64), // in mb. must also store geopotential height in m
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
            MBAR(p, _h) => write!(f, "{p} mb"),
        }
    }
}

impl Layer {
    fn height_agl(&self, station_altitude: Altitude) -> Altitude {
        let height = match self {
            All => Altitude::new(f32::NAN, Meter),
            Indoor => Altitude::new(1., Meter),
            NearSurface => Altitude::new(2., Meter),
            SeaLevel => station_altitude*-1.,
            AGL(a) => Altitude::new(*a as f32, Meter),
            MSL(a) => Altitude::new(*a as f32, Meter) - station_altitude,
            MBAR(_p, a) => Altitude::new(*a as f32, Meter)
        };

        height
    }
}



#[derive(Debug, Clone, Copy, Serialize)]
pub struct Wind {
    pub direction: Direction, // stored as degrees
    pub speed: Speed,
}

impl Display for Wind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}°@{} kts", self.direction.degrees(), self.speed)
    }
}


#[derive(Serialize, Debug, Clone, Copy, Display, PartialEq)]
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


#[derive(Serialize, Debug, Clone, Copy)]
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

#[derive(Serialize, Debug, Clone)]
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

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Precip {
    pub unknown: PrecipAmount,
    pub rain: PrecipAmount,
    pub snow: PrecipAmount,
}

impl Display for Precip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rain: {}, Snow: {}, Unknown: {}", self.rain, self.snow, self.unknown)
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

// HASHMAP BASED IMPLEMENTATION ------------------------------------------------
#[derive(Debug)]
pub struct HashMapWx {
    date_time: DateTime<Utc>,
    station: Station,
    data: HashMap<(Layer, Param), Box<dyn Any>>
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

impl<'a> WxEntry<'a, LayerHash<'a>> for HashMapWx {
    fn date_time(&self) -> DateTime<Utc> {self.date_time}
    fn station(&self) -> Station {self.station.clone()}
    fn layer(&'a self, layer: Layer) -> Option<LayerHash<'a>> {Some(LayerHash {layer, data: self})}
    fn layers(&self) -> Vec<Layer> {
        let mut set = HashSet::new();
        for entry in self.data.keys() {
            set.insert(entry.0);
        }
        set.iter().map(|x| x.to_owned()).collect()
    }

    fn skycover(&self) -> Option<SkyCoverage> {self.get(All, Param::SkyCover)}
    fn wx_codes(&self) -> Option<Vec<String>> {self.get(All, Param::WxCodes)}
    fn raw_metar(&self) -> Option<String> {self.get(All, Param::RawMetar)}
    fn precip_today(&self) -> Option<Precip> {self.get(All, Param::PrecipToday)}
    fn precip(&self) -> Option<Precip> {self.get(All, Param::Precip)}
    fn altimeter(&self) -> Option<Pressure> {self.get(All, Param::Altimeter)}
    fn cape(&self) -> Option<SpecEnergy> {self.get(All, Param::Cape)} 
}


impl HashMapWx {
    pub fn new(date_time: DateTime<Utc>, station: Station) -> HashMapWx {
        HashMapWx { date_time, station, data: HashMap::new() }
    }
    pub fn put<U: Clone + 'static>(&mut self, layer: Layer, param: Param, data: U) {
        self.data.insert((layer, param), Box::new(data));
    }
    // same as put, but if there is None, don't insert anything
    pub fn put_opt<U: Clone + 'static>(&mut self, layer: Layer, param: Param, data: Option<U>) {
        if let Some(d) = data {
            self.data.insert((layer, param), Box::new(d));
        }
    }
    pub fn get<U: Clone + 'static>(&self, layer: Layer, param: Param) -> Option<U> {
        Some(self.data.get(&(layer, param))?.downcast_ref::<U>()?.clone())
    }
}

pub struct LayerHash<'a> {
    layer: Layer,
    data: &'a HashMapWx
} 

impl<'a> LayerHash<'a> {
    fn get<U: Copy + 'static>(&self, param: Param) -> Option<U> {
        self.data.get(self.layer, param)
    }
}

impl<'a> WxEntryLayer for LayerHash<'a> {
    fn layer(&self) -> Layer {self.layer}
    fn station(&self) -> Station {self.data.station.clone()}

    fn temperature(&self) -> Option<Temperature> {self.get(Param::Temperature)}
    fn pressure(&self) -> Option<Pressure> {self.get(Param::Pressure)}
    fn visibility(&self) -> Option<Distance> {self.get(Param::Visibility)}

    fn dewpoint(&self) -> Option<Temperature> {self.get(Param::Dewpoint).or(self.dewpoint_from_rh())}
    fn relative_humidity(&self) -> Option<Fraction> {self.get(Param::RelativeHumidity).or(self.rh_from_dewpoint())}

    fn wind_speed(&self) -> Option<Speed> {self.get(Param::WindSpeed).or(self.wind_speed_from_wind())}
    fn wind_direction(&self) -> Option<Direction> {self.get(Param::WindDirection).or(self.wind_direction_from_wind())}
    fn wind(&self) -> Option<Wind> {self.get(Param::Wind).or(self.wind_from_speed_and_direction())}
}



// BASIC WXENTRY IMPLEMENTATION ------------------------------------------------
// Can provide a consistent struct to handle

#[derive(Debug, Clone, Serialize)]
pub struct WxEntryStruct {
    date_time: DateTime<Utc>,
    station: Station,
    layers: HashMap<Layer, WxEntryLayerStruct>,
    
    skycover: Option<SkyCoverage>,
    wx_codes: Option<Vec<String>>,
    raw_metar: Option<String>,
    precip_today: Option<Precip>,
    precip: Option<Precip>,
    altimeter: Option<Pressure>,
    cape: Option<SpecEnergy>,
}

impl<'a> WxEntry<'a, &'a WxEntryLayerStruct> for WxEntryStruct {
    fn date_time(&self) -> DateTime<Utc> {self.date_time}
    fn station(&self) -> Station {self.station.clone()}
    fn layer(&'a self, layer: Layer) -> Option<&'a WxEntryLayerStruct> {
        self.layers.get(&layer)
    }
    fn layers(&self) -> Vec<Layer> {self.layers.keys().map(|x| x.to_owned()).collect()}

    fn skycover(&self)     -> Option<SkyCoverage> {self.skycover.clone()}
    fn wx_codes(&self)     -> Option<Vec<String>> {self.wx_codes.clone()}
    fn raw_metar(&self)    -> Option<String>      {self.raw_metar.clone()}
    fn precip_today(&self) -> Option<Precip>      {self.precip_today}
    fn precip(&self)       -> Option<Precip>      {self.precip}
    fn altimeter(&self)    -> Option<Pressure>    {self.altimeter}
    fn cape(&self)         -> Option<SpecEnergy>  {self.cape} 
}

#[derive(Debug, Clone, Serialize)]
pub struct WxEntryLayerStruct {
    layer: Layer,
    station: Station,
    temperature: Option<Temperature>,
    pressure: Option<Pressure>,
    visibility: Option<Distance>,
    wind: Option<Wind>,
    dewpoint: Option<Temperature>,
}

impl<'a> WxEntryLayer for &'a WxEntryLayerStruct {
    fn layer(&self) -> Layer {self.layer}
    fn station(&self) -> Station {self.station.clone()}
    fn temperature(&self) -> Option<Temperature> {self.temperature}
    fn pressure(&self) -> Option<Pressure> {self.pressure}
    fn visibility(&self) -> Option<Distance> {self.visibility}
    fn dewpoint(&self) -> Option<Temperature> {self.dewpoint}
    fn wind(&self) -> Option<Wind> {self.wind}
}

//


#[cfg(test)]
mod tests {
    use crate::*;
    use crate::Layer::*;

    struct TestLayer {
        layer: Layer,
        station: Station,
        temperature: Option<Temperature>, 
        wind_speed: Option<Speed>,
        dewpoint: Option<Temperature>,
    }

    impl WxEntryLayer for TestLayer {
        fn layer(&self) -> Layer {self.layer}
        fn station(&self) -> Station {self.station.clone()}
    }

    fn default_station() -> Station {
        Station {
            name: String::from("Test"),
            altitude: Altitude::new(0., Meter),
            coords: (0., 0.)
        }
    }


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

        let mut e = TestLayer { 
            layer: NearSurface, 
            station: default_station(),
            dewpoint: None,
            temperature: None,
            wind_speed: None
        };

        e.temperature = Some(Temperature::new(51., Fahrenheit));
        assert_eq!(e.apparent_temp(), Some(Temperature::new(51., Fahrenheit)));

        e.temperature = Some(Temperature::new(49., Fahrenheit));
        assert_eq!(e.apparent_temp(), None);

        e.wind_speed = Some(Speed::new(2., Mph));
        assert_eq!(e.apparent_temp(), Some(Temperature::new(49., Fahrenheit)));

        e.temperature = Some(Temperature::new(79., Fahrenheit));
        assert_eq!(e.apparent_temp(), Some(Temperature::new(79., Fahrenheit)));

        e.temperature = Some(Temperature::new(81., Fahrenheit));
        assert_eq!(e.apparent_temp(), None);

        e.dewpoint = Some(Temperature::new(54., Fahrenheit));
        assert_eq!(e.apparent_temp(), Some(Temperature::new(81., Fahrenheit)));


        // heat index tests

        e.temperature = Some(Temperature::new(81., Fahrenheit));
        e.dewpoint = Some(Temperature::new(65., Fahrenheit));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(apparent_temp.value_in(Fahrenheit), 82.8));

        e.temperature = Some(Temperature::new(100., Fahrenheit));
        e.dewpoint = Some(Temperature::new(75., Fahrenheit));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(apparent_temp.value_in(Fahrenheit), 113.7));

        e.temperature = Some(Temperature::new(110., Fahrenheit));
        e.dewpoint = Some(Temperature::new(85., Fahrenheit));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(apparent_temp.value_in(Fahrenheit), 146.1));


        // wind chill tests

        e.temperature = Some(Temperature::new(32., Fahrenheit));
        e.wind_speed = Some(Speed::new(10., Mph));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(apparent_temp.value_in(Fahrenheit), 23.0));

        e.temperature = Some(Temperature::new(49., Fahrenheit));
        e.wind_speed = Some(Speed::new(3., Mph));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(apparent_temp.value_in(Fahrenheit), 48.1));

        e.temperature = Some(Temperature::new(49., Fahrenheit));
        e.wind_speed = Some(Speed::new(40., Mph));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(apparent_temp.value_in(Fahrenheit), 38.9));

        e.temperature = Some(Temperature::new(-20., Fahrenheit));
        e.wind_speed = Some(Speed::new(3., Mph));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(apparent_temp.value_in(Fahrenheit), -30.7));


        e.temperature = Some(Temperature::new(-20., Fahrenheit));
        e.wind_speed = Some(Speed::new(40., Mph));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(apparent_temp.value_in(Fahrenheit), -58.4));

    }
}
