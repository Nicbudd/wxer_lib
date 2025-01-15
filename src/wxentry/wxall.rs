use std::collections::HashMap;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Debug, Serialize)]
pub struct DataWithStation<'a, T: Serialize> {
    pub station: &'a Station,
    pub data: T
}


#[derive(Debug, Deserialize)]
pub struct DataWithStationDeserialize<T> {
    pub station: Station,
    pub data: T
}

#[derive(Debug, Clone, Serialize)]
pub struct WxAll {
    // #[serde(skip_serializing)]
    pub date_time: DateTime<Utc>,
    pub date_time_local: DateTime<Tz>,
    // #[serde(skip_serializing)]
    pub station: &'static Station,
    pub layers: HashMap<Layer, WxAllLayer>,

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
    pub altimeter: Option<Pressure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cape: Option<SpecEnergy>,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_slp: Option<Pressure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WxAllLayer {
    // #[serde(skip_serializing)]
    layer: Layer,
    // #[serde(skip_serializing)]
    station: &'static Station,

    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<Temperature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pressure: Option<Pressure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visibility: Option<Distance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wind: Option<WindExpanded>,

    #[serde(skip_serializing_if = "Option::is_none")]
    dewpoint: Option<Temperature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    relative_humidity: Option<Fraction>,

    #[serde(skip_serializing_if = "Option::is_none")]
    projected_slp: Option<Pressure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wind_chill_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    wind_chill: Option<Temperature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    heat_index_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    heat_index: Option<Temperature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    apparent_temp: Option<Temperature>,
    #[serde(skip_serializing_if = "Option::is_none")]
    theta_e: Option<Temperature>,

}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct UnitPreferences {
    pub temperature: TemperatureUnit,
    pub pressure: PressureUnit,
    pub distance: DistanceUnit,
    pub speed: SpeedUnit,
    pub theta_e: TemperatureUnit,
}

impl Default for UnitPreferences {
    fn default() -> Self {
        UnitPreferences { 
            temperature: Fahrenheit, 
            pressure: Mbar, 
            distance: Mile, 
            speed: Knots,
            theta_e: Kelvin
        }
    }
}

impl WxAll {
    pub fn new(wx: &WxEntryStruct, units: UnitPreferences) -> WxAll {
        let mut layers = HashMap::new();

        for (layer, s) in &wx.layers {
            let l = WxAllLayer { 
                layer: *layer,
                station: s.station(), 
                temperature: s.temperature().map(|x| x.convert(units.temperature)), 
                pressure: s.pressure().map(|x| x.convert(units.pressure)), 
                visibility: s.visibility().map(|x| x.convert(units.distance)), 
                wind: s.wind().map(|wind|
                        Wind {
                            speed: wind.speed.convert(units.speed), 
                            direction: wind.direction
                        }.into()
                    ),
                dewpoint: s.dewpoint().map(|x| x.convert(units.temperature)), 
                relative_humidity: s.relative_humidity().map(|x| x.convert(Percent)), 
                projected_slp: s.slp().map(|x| x.convert(units.pressure)), 
                wind_chill_valid: s.wind_chill_valid(), 
                wind_chill: s.wind_chill().map(|x| x.convert(units.temperature)), 
                heat_index_valid:  s.heat_index_valid(), 
                heat_index: s.heat_index().map(|x| x.convert(units.temperature)), 
                apparent_temp: s.apparent_temp().map(|x| x.convert(units.temperature)), 
                theta_e: s.theta_e(wx.altimeter()).map(|x| x.convert(units.theta_e)) 
            };
            layers.insert(*layer, l);
        }

        let wx = WxAll { 
            date_time: wx.date_time(), 
            date_time_local: wx.date_time_local(), 
            station: wx.station(), 
            layers, 
            skycover: wx.skycover(), 
            wx_codes: wx.wx_codes(), 
            wx: wx.wx(), 
            raw_metar: wx.raw_metar(), 
            precip_today: wx.precip_today(), 
            precip: wx.precip(), 
            altimeter: wx.altimeter(), 
            cape: wx.cape(), 
            best_slp: wx.best_slp().map(|x| x.convert(units.pressure))
        };
        
        wx
    }
}

impl<'a> WxEntry<'a, &'a WxAllLayer> for WxAll {
    fn date_time(&self) -> chrono::DateTime<chrono::Utc> {self.date_time}
    #[allow(refining_impl_trait)]
    fn station(&self) -> &'static Station {self.station}
    fn layer(&'a self, layer: Layer) -> Option<&WxAllLayer> {self.layers.get(&layer)}
    fn layers(&self) -> Vec<Layer> {self.layers.iter().map(|x| x.0.to_owned()).collect()}

    fn skycover(&self) -> Option<SkyCoverage> {self.skycover.clone()}
    fn wx_codes(&self) -> Option<Vec<String>> {self.wx_codes.clone()}
    fn raw_metar(&self) -> Option<String> {self.raw_metar.clone()}
    fn precip_today(&self) -> Option<Precip> {self.precip_today}
    fn precip(&self) -> Option<Precip> {self.precip}
    fn altimeter(&self) -> Option<Pressure> {self.altimeter}
    fn cape(&self) -> Option<SpecEnergy> {self.cape}
}

impl<'a> WxEntryLayer for &'a WxAllLayer {
    fn layer(&self) -> Layer {self.layer}
    #[allow(refining_impl_trait)]
    fn station(&self) -> &'static Station {self.station}

    fn temperature(&self) -> Option<Temperature> {self.temperature}
    fn pressure(&self) -> Option<Pressure> {self.pressure}
    fn visibility(&self) -> Option<Distance> {self.visibility}
    fn wind(&self) -> Option<Wind> {Some(self.wind.clone()?.into())}

    fn dewpoint(&self) -> Option<Temperature> {self.dewpoint}
}



#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindExpanded {
    pub direction: DirectionExpanded,
    pub speed: Speed,
}

impl From<Wind> for WindExpanded {
    fn from(value: Wind) -> Self {
        WindExpanded { 
            direction: value.direction.into(), 
            speed: value.speed 
        }
    }
}

impl From<WindExpanded> for Wind {
    fn from(value: WindExpanded) -> Self {
        Wind { 
            direction: value.direction.into(), 
            speed: value.speed 
        }
    }
}