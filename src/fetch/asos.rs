use std::collections::{BTreeMap, HashMap};

use crate::{db::StationData, ignore_none, CloudLayer, Direction, Layer, Precip, SkyCoverage, Station, WxEntry, WxEntryLayer};

use chrono::{DateTime, Duration, Timelike, Utc};
use serde::Deserialize;
use anyhow::{bail, Result};

pub async fn import(station_name: &str, network: &str, station: Station) -> Result<StationData> {
    let url = format!("http://mesonet.agron.iastate.edu/json/current.py?station={}&network={}", station_name, network);

    //dbg!(&url);

    // let psm_station: Station = Station {
    //     coords: (43.08, -70.82),
    //     altitude: 30.,
    //     name: String::from("KPSM"),
    // };  

    // let mht_station: Station = Station {
    //     coords: (42.93, -71.43),
    //     altitude: 81.,
    //     name: String::from("KMHT"),
    // };   

    // let unknown_station: Station = Station { 
    //     name: "UNKNOWN".into(), 
    //     altitude: (f32::NAN), 
    //     coords: (f32::NAN, f32::NAN) 
    // };

    // let station = match (station_name, network) {
    //     ("PSM", "NH_ASOS") => {psm_station},
    //     ("MHT", "NH_ASOS") => {mht_station},
    //     (name, _) => {let mut s = unknown_station.clone(); s.name = String::from(name); s},
    // };

    let resp: String = reqwest::get(url)
        .await?
        .text()
        .await?;

    let raw_ob: RawASOSObservation = serde_json::from_str(&resp)?;

    let mut dt = raw_ob.last_ob.utc_valid.parse::<DateTime<Utc>>()?;
    dt -= Duration::seconds(dt.second() as i64); // round to previous minute

    let skycover = Some(skycover_from_vecs(raw_ob.last_ob.skycover, raw_ob.last_ob.skylevel)?);

    let wind_direction = match raw_ob.last_ob.winddirectiondeg {
        Some(dir) => Some(Direction::from_degrees(dir as u16)?),
        None => None
    };

    let precip_today = ignore_none(raw_ob.last_ob.precip_today, |x| {
        Precip{unknown: x, rain: 0., snow: 0.}
    });

    let mut asos_db = BTreeMap::new();

    let near_surface = WxEntryLayer { 
        layer: Layer::NearSurface, 
        height_agl: Some(2.0), 
        height_msl: Some(station.altitude), 
        temperature: raw_ob.last_ob.airtempF, 
        dewpoint: raw_ob.last_ob.dewpointtempF, 
        pressure: None, 
        wind_direction, 
        wind_speed: raw_ob.last_ob.windspeedkt, 
        visibility: raw_ob.last_ob.visibilitymile,
    };

    let sea_level = WxEntryLayer { 
        layer: Layer::SeaLevel, 
        height_agl: None, 
        height_msl: Some(0.0), 
        temperature: None, 
        dewpoint: None, 
        pressure: raw_ob.last_ob.mslpmb, 
        wind_direction: None, 
        wind_speed: None, 
        visibility: None,
    };

    let mut layers = HashMap::new();

    layers.insert(Layer::NearSurface, near_surface);
    layers.insert(Layer::SeaLevel, sea_level);

    let entry: WxEntry = WxEntry { 
        date_time: dt,
        station: station.clone(),

        layers,

        cape: None,
        skycover,
        raw_metar: raw_ob.last_ob.raw,
        precip_today,
        precip: None,
        precip_probability: None,
        present_wx: None
    };

    asos_db.insert(dt, entry);

    Ok(asos_db)
}



#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct RawASOSObservation {
    id: String,
    network: String,
    last_ob: ASOSOb,
}

#[allow(dead_code, non_snake_case)]
#[derive(Debug, Deserialize)]
struct ASOSOb {
    utc_valid: String,    

    #[serde(rename="airtemp[F]")]
    airtempF: Option<f32>,

    #[serde(rename="max_dayairtemp[F]")]
    max_dayairtempF: Option<f32>,

    #[serde(rename="min_dayairtemp[F]")]
    min_dayairtempF: Option<f32>,

    #[serde(rename="dewpointtemp[F]")]
    dewpointtempF: Option<f32>,

    #[serde(rename="windspeed[kt]")]
    windspeedkt: Option<f32>,

    #[serde(rename="winddirection[deg]")]
    winddirectiondeg: Option<f32>,

    #[serde(rename="altimeter[in]")]
    altimeterin: Option<f32>,

    #[serde(rename="mslp[mb]")]
    mslpmb: Option<f32>,

    #[serde(rename="skycover[code]")]
    skycover: Vec<Option<String>>,

    #[serde(rename="skylevel[ft]")]
    skylevel: Vec<Option<u32>>,

    #[serde(rename="visibility[mile]")]
    visibilitymile: Option<f32>,

    raw: Option<String>,

    #[serde(rename="presentwx")]
    present_wx: Option<Vec<String>>,

    #[serde(rename="precip_today[in]")]
    precip_today: Option<f32>,

    #[serde(rename="cltmpf[F]")]
    cltmpf: Option<f32>,

}

fn skycover_from_vecs(cover: Vec<Option<String>>, level: Vec<Option<u32>>) -> Result<SkyCoverage> {
    
    if level.iter().filter(|x: &&Option<u32>| x.is_some()).count() == 0 {
        return Ok(SkyCoverage::Clear)
    }

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
            _ => {bail!("Mismatched skycover and skylevel values")}
        }
    }
 
    Ok(SkyCoverage::Cloudy(skyc))
}

