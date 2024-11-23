
use std::{any::Any, collections::{HashMap, HashSet}, sync::Arc};

use chrono::{DateTime, Utc};

use crate::*;

#[derive(Debug)]
pub struct HashMapWx {
    date_time: DateTime<Utc>,
    station: Arc<Station>,
    data: HashMap<(Layer, Param), Box<dyn Any>>
}

impl<'a> WxEntry<'a, LayerHash<'a>> for HashMapWx {
    fn date_time(&self) -> DateTime<Utc> {self.date_time}
    #[allow(refining_impl_trait)]
    fn station(&self) -> Arc<Station> {self.station.clone()}
    fn layer(&'a self, layer: Layer) -> Option<LayerHash> {Some(LayerHash {layer, data: self})}
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
    pub fn new(date_time: DateTime<Utc>, station: Arc<Station>) -> HashMapWx {
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
    #[allow(refining_impl_trait)]
    fn station(&self) -> Arc<Station> {self.data.station.clone()}

    fn temperature(&self) -> Option<Temperature> {self.get(Param::Temperature)}
    fn pressure(&self) -> Option<Pressure> {self.get(Param::Pressure)}
    fn visibility(&self) -> Option<Distance> {self.get(Param::Visibility)}

    fn dewpoint(&self) -> Option<Temperature> {
        self.get(Param::Dewpoint).or({
            Some(dewpoint_from_rh(self.get(Param::Temperature)?, self.get(Param::RelativeHumidity)?))
        })
    }
    fn relative_humidity(&self) -> Option<Fraction> {
        self.get(Param::RelativeHumidity).or({
            Some(rh_from_dewpoint(self.get(Param::Temperature)?, self.get(Param::Dewpoint)?))
        })
    }

    fn wind_speed(&self) -> Option<Speed> {
        self.get(Param::WindSpeed).or(
            Some(self.get::<Wind>(Param::Wind)?.speed)
        )
    }
    fn wind_direction(&self) -> Option<Direction> {
        self.get(Param::WindDirection).or(
            Some(self.get::<Wind>(Param::Wind)?.direction)
        )
    }
    fn wind(&self) -> Option<Wind> {
        self.get(Param::Wind).or(
            Some(Wind {direction: self.get(Param::WindDirection)?, speed: self.get(Param::WindSpeed)?})
        )
    }
}