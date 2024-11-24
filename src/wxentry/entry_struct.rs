use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::*;

#[derive(Debug, Clone, Serialize)]
pub struct WxEntryStruct {
    pub date_time: DateTime<Utc>,
    #[serde(skip_serializing)]
    pub station: &'static Station,
    pub layers: HashMap<Layer, WxEntryLayerStruct>,
    
    pub skycover: Option<SkyCoverage>,
    pub wx_codes: Option<Vec<String>>,
    pub raw_metar: Option<String>,
    pub precip_today: Option<Precip>,
    pub precip: Option<Precip>,
    pub altimeter: Option<Pressure>,
    pub cape: Option<SpecEnergy>,
}

impl<'a> WxEntry<'a, &'a WxEntryLayerStruct> for WxEntryStruct {
    fn date_time(&self) -> DateTime<Utc> {self.date_time}
    #[allow(refining_impl_trait)]
    fn station(&self) -> &'static Station {self.station}
    #[allow(refining_impl_trait)]
    fn layer(&'a self, layer: Layer) -> Option<&WxEntryLayerStruct> {
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
    pub layer: Layer,
    #[serde(skip_serializing)]
    pub station: &'static Station,
    pub temperature: Option<Temperature>,
    pub pressure: Option<Pressure>,
    pub visibility: Option<Distance>,
    pub wind: Option<Wind>,
    pub dewpoint: Option<Temperature>,
}

impl<'a> WxEntryLayer for &'a WxEntryLayerStruct {
    fn layer(&self) -> Layer {self.layer}
    #[allow(refining_impl_trait)]
    fn station(&self) -> &'static Station {self.station}
    fn temperature(&self) -> Option<Temperature> {self.temperature}
    fn pressure(&self) -> Option<Pressure> {self.pressure}
    fn visibility(&self) -> Option<Distance> {self.visibility}
    fn dewpoint(&self) -> Option<Temperature> {self.dewpoint}
    fn wind(&self) -> Option<Wind> {self.wind}
}
