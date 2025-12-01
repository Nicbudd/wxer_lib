use std::collections::{BTreeMap, HashMap};

use crate::Layer::*;
use crate::*;
#[allow(unused)]
use log::{debug, error, info, trace, warn};
// use crate::{db::StationData, ignore_none, CloudLayer, Direction, Layer, Precip, SkyCoverage, Station, WxEntry, WxEntryLayer};

use anyhow::{bail, Result};
use chrono::{DateTime, Duration, Timelike, Utc};
use serde::Deserialize;

pub async fn import(
    station_name: &str,
    network: &str,
    station: &'static Station,
) -> Result<db::StationData> {
    let url = format!(
        "http://mesonet.agron.iastate.edu/json/current.py?station={}&network={}",
        station_name, network
    );

    let resp: String = reqwest::get(url).await?.text().await?;

    let raw_ob: RawASOSObservation = serde_json::from_str(&resp)?;

    let ob = raw_ob.last_ob;

    let mut date_time = ob.utc_valid.parse::<DateTime<Utc>>()?;
    date_time -= Duration::seconds(date_time.second() as i64); // round to previous minute

    // let mut wx_entry = HashMapWx::new(dt, station);

    let precip_today = ignore_none(ob.precip_today, |x| Precip {
        unknown: PrecipAmount::new(x, Inch),
        rain: PrecipAmount::new(0., Inch),
        snow: PrecipAmount::new(0., Inch),
    });

    fn make_wind(speed: Option<f32>, dir: Option<f32>) -> Option<Wind> {
        let speed = Speed::new(speed?, Knots);
        let direction = Direction::from_degrees(dir? as u16).ok();
        Some(Wind { direction, speed })
    }

    let temperature = ob.airtempF.map(|x| Temperature::new(x, Fahrenheit));
    let dewpoint = ob.dewpointtempF.map(|x| Temperature::new(x, Fahrenheit));
    let visibility = ob.visibilitymile.map(|x| Distance::new(x, Mile));
    let pressure = ob.mslpmb.map(|x| Pressure::new(x, HPa));

    let wind = make_wind(ob.windspeedkt, ob.winddirectiondeg);
    // let precip_

    let near_surface = WxEntryLayerStruct {
        layer: NearSurface,
        station,
        temperature,
        pressure: None,
        visibility,
        wind,
        dewpoint,
        height_msl: NearSurface.height_agl(Altitude::new(0.0, Meter)),
    };

    let sea_level = WxEntryLayerStruct {
        layer: SeaLevel,
        station,
        temperature: None,
        pressure,
        visibility: None,
        wind: None,
        dewpoint: None,
        height_msl: None,
    };

    let mut layers = HashMap::new();
    layers.insert(NearSurface, near_surface);
    layers.insert(SeaLevel, sea_level);

    let altimeter = ob.altimeterin.map(|x| Pressure::new(x, InHg));
    let skycover = skycover_from_vecs(ob.skycover, ob.skylevel).ok();

    let wx_entry = WxEntryStruct {
        date_time,
        station,
        layers,
        altimeter,
        skycover,
        cape: None,
        precip: None,
        precip_probability: None,
        precip_today,
        wx_codes: ob.present_wx,
        raw_metar: ob.raw,
    };

    // let d = wx_entry.get::<Temperature>(NearSurface, Param::Dewpoint);
    // let dew = wx_entry.surface().unwrap().dewpoint();
    // dbg!(d, dew);

    let mut asos_db = BTreeMap::new();

    let as_struct = wx_entry.to_struct()?;
    // dbg!(&as_struct);

    asos_db.insert(date_time, as_struct);

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

    #[serde(rename = "airtemp[F]")]
    airtempF: Option<f32>,

    #[serde(rename = "max_dayairtemp[F]")]
    max_dayairtempF: Option<f32>,

    #[serde(rename = "min_dayairtemp[F]")]
    min_dayairtempF: Option<f32>,

    #[serde(rename = "dewpointtemp[F]")]
    dewpointtempF: Option<f32>,

    #[serde(rename = "windspeed[kt]")]
    windspeedkt: Option<f32>,

    #[serde(rename = "winddirection[deg]")]
    winddirectiondeg: Option<f32>,

    #[serde(rename = "altimeter[in]")]
    altimeterin: Option<f32>,

    #[serde(rename = "mslp[mb]")]
    mslpmb: Option<f32>,

    #[serde(rename = "skycover[code]")]
    skycover: Vec<Option<String>>,

    #[serde(rename = "skylevel[ft]")]
    skylevel: Vec<Option<u32>>,

    #[serde(rename = "visibility[mile]")]
    visibilitymile: Option<f32>,

    raw: Option<String>,

    #[serde(rename = "presentwx")]
    present_wx: Option<Vec<String>>,

    #[serde(rename = "precip_today[in]")]
    precip_today: Option<f32>,

    #[serde(rename = "cltmpf[F]")]
    cltmpf: Option<f32>,
    // #[serde(skip_deserializing)]
    // date_time: DateTime<Utc>,

    // #[serde(skip_deserializing)]
    // station: Station,
}

fn skycover_from_vecs(cover: Vec<Option<String>>, level: Vec<Option<u32>>) -> Result<SkyCoverage> {
    if level.iter().filter(|x: &&Option<u32>| x.is_some()).count() == 0 {
        return Ok(SkyCoverage::Clear);
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
            _ => {
                bail!("Mismatched skycover and skylevel values")
            }
        }
    }

    Ok(SkyCoverage::Cloudy(skyc))
}
