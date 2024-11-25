use std::collections::BTreeMap;

use crate::*;
use crate::Layer::*;
#[allow(unused)]
use log::{error, warn, info, debug, trace};
// use crate::{db::StationData, ignore_none, CloudLayer, Direction, Layer, Precip, SkyCoverage, Station, WxEntry, WxEntryLayer};

use chrono::{DateTime, Duration, Timelike, Utc};
use serde::Deserialize;
use anyhow::{bail, Result};

pub async fn import(station_name: &str, network: &str, station: &'static Station) -> Result<db::StationData> {
    let url = format!("http://mesonet.agron.iastate.edu/json/current.py?station={}&network={}", station_name, network);

    //dbg!(&url);

    let resp: String = reqwest::get(url)
        .await?
        .text()
        .await?;

    let raw_ob: RawASOSObservation = serde_json::from_str(&resp)?;

    let ob = raw_ob.last_ob;
    
    let mut dt = ob.utc_valid.parse::<DateTime<Utc>>()?;
    dt -= Duration::seconds(dt.second() as i64); // round to previous minute
    
    let mut wx_entry = HashMapWx::new(dt, station);

    let precip_today = ignore_none(ob.precip_today, |x| {
        Precip {
            unknown: PrecipAmount::new(x, Inch), 
            rain: PrecipAmount::new(0., Inch), 
            snow: PrecipAmount::new(0., Inch),
        }
    });

    let direction = ob.winddirectiondeg.map(|x| Direction::from_degrees(x as u16).ok()).flatten();

    wx_entry.put(All, Param::SkyCover, skycover_from_vecs(ob.skycover, ob.skylevel)?);

    if let Some(x) = direction {
        wx_entry.put(All, Param::WindDirection, x);
    }
    if let Some(x) = precip_today{
        wx_entry.put(All, Param::PrecipToday, x);
    }
    if let Some(x) = ob.raw {
        wx_entry.put(All, Param::RawMetar, x);
    }
    if let Some(x) = ob.present_wx {
        wx_entry.put(All, Param::WxCodes, x);
    }

    if let Some(x) = ob.airtempF{
        wx_entry.put(NearSurface, Param::Temperature, Temperature::new(x, Fahrenheit));
    }
    if let Some(x) = ob.dewpointtempF {
        wx_entry.put(NearSurface, Param::Dewpoint, Temperature::new(x, Fahrenheit));
    }
    if let Some(x) = ob.windspeedkt {
        wx_entry.put(NearSurface, Param::WindSpeed, Speed::new(x, Knots));
    }
    if let Some(x) = ob.winddirectiondeg {
        let dir = Direction::from_degrees(x as u16);
        if let Ok(y) = dir {
            wx_entry.put(NearSurface, Param::WindDirection, y);
        } else {
            warn!("{} ASOS: Failed to convert {x} into degrees", station_name)
        }
    }
    if let Some(x) = ob.altimeterin {
        wx_entry.put(NearSurface, Param::Altimeter, Pressure::new(x, InHg));
    }
    if let Some(x) = ob.visibilitymile {
        wx_entry.put(NearSurface, Param::Visibility, Distance::new(x, Mile));
    }
    if let Some(x) = ob.altimeterin {
        wx_entry.put(SeaLevel, Param::Pressure,  Pressure::new(x, InHg));
    }

    let mut asos_db = BTreeMap::new();
    
    asos_db.insert(dt, wx_entry.to_struct()?);


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

    // #[serde(skip_deserializing)]
    // date_time: DateTime<Utc>,

    // #[serde(skip_deserializing)]
    // station: Station,

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

