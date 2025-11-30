use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::*;

#[derive(Debug, Clone, Deserialize)]
pub struct WxStructDeserialized {
    pub date_time: DateTime<Utc>,
    pub station: Station,
    pub layers: HashMap<Layer, WxStructDeserializedLayer>,

    pub skycover: Option<SkyCoverage>,
    pub wx_codes: Option<Vec<String>>,
    pub raw_metar: Option<String>,
    pub precip_today: Option<Precip>,
    pub precip: Option<Precip>,
    pub altimeter: Option<Pressure>,
    pub cape: Option<SpecEnergy>,
}

impl<'a> WxEntry<'a, &'a WxStructDeserializedLayer> for WxStructDeserialized {
    fn date_time(&self) -> DateTime<Utc> {
        self.date_time
    }
    fn station(&self) -> &'static Station {
        let l: &'static Station = Box::leak(Box::new(self.station.clone()));
        l
    }
    fn layer(&'a self, layer: Layer) -> Option<&'a WxStructDeserializedLayer> {
        self.layers.get(&layer)
    }
    fn layers(&self) -> Vec<Layer> {
        self.layers.keys().map(|x| x.to_owned()).collect()
    }

    fn skycover(&self) -> Option<SkyCoverage> {
        self.skycover.clone()
    }
    fn wx_codes(&self) -> Option<Vec<String>> {
        self.wx_codes.clone()
    }
    fn raw_metar(&self) -> Option<String> {
        self.raw_metar.clone()
    }
    fn precip_today(&self) -> Option<Precip> {
        self.precip_today
    }
    fn precip(&self) -> Option<Precip> {
        self.precip
    }
    fn altimeter(&self) -> Option<Pressure> {
        self.altimeter
    }
    fn cape(&self) -> Option<SpecEnergy> {
        self.cape
    }
}

// impl From<WxStructDeserializedLayer> for WxEntryLayerStruct {
//     fn from(value: WxStructDeserializedLayer) -> Self {
//         WxEntryLayerStruct {

//         }
//     }
// }

#[derive(Debug, Clone, Deserialize)]
pub struct WxStructDeserializedLayer {
    pub layer: Layer,
    pub station: Station,
    pub temperature: Option<Temperature>,
    pub pressure: Option<Pressure>,
    pub visibility: Option<Distance>,
    pub wind: Option<WindExpanded>,
    pub dewpoint: Option<Temperature>,
}

impl WxEntryLayer for &WxStructDeserializedLayer {
    fn layer(&self) -> Layer {
        self.layer
    }
    #[allow(refining_impl_trait)]
    fn station(&self) -> &'static Station {
        let l: &'static Station = Box::leak(Box::new(self.station.clone()));
        l
    }
    fn temperature(&self) -> Option<Temperature> {
        self.temperature
    }
    fn pressure(&self) -> Option<Pressure> {
        self.pressure
    }
    fn visibility(&self) -> Option<Distance> {
        self.visibility
    }
    fn dewpoint(&self) -> Option<Temperature> {
        self.dewpoint
    }
    fn wind(&self) -> Option<Wind> {
        Some(self.wind.clone()?.into())
    }
}
