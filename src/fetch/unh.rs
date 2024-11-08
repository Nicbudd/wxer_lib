use std::collections::{BTreeMap, HashMap};

use crate::{db::StationData, rh_to_dewpoint, Direction, Layer, Precip, Station, WxEntry, WxEntryLayer};

use chrono::{offset::LocalResult, DateTime, Datelike, Local, NaiveDateTime, TimeZone, Utc};
use chrono_tz::US::Eastern;
use serde::Deserialize;
use anyhow::Result;

pub async fn import(date: DateTime<Utc>) -> Result<StationData> {

    let day = date.with_timezone(&Eastern).ordinal();
    let year = date.with_timezone(&Eastern).year();

    let url = format!("https://www.weather.unh.edu/data/{year}/{day}.txt");
    // dbg!(&url);

    let unh_text = reqwest::get(&url).await?.text().await?;

    let mut rdr = csv::Reader::from_reader(unh_text.as_bytes());

    let mut db = BTreeMap::new();

    for entry_result in rdr.deserialize() {
        let entry: UNHWxEntry = entry_result?;
        let wx_entry: WxEntry = entry.to_wx_entry();

        db.insert(wx_entry.date_time, wx_entry);
    }

    Ok(db)

    // Ok(result_map)
}

fn deserialize_unh_dt<'de, D>(des: D) -> Result<DateTime<Utc>, D::Error> 
    where D: serde::Deserializer<'de> {

    let s = String::deserialize(des)?;

    let dt_naive = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").map_err(serde::de::Error::custom)?;

    let local_result: LocalResult<DateTime<Local>> = Local.from_local_datetime( &dt_naive); 

    let dt_local = match local_result {
        LocalResult::None => {DateTime::default()}
        LocalResult::Single(a) => a,
        LocalResult::Ambiguous(a, _) => a, // idc 
    };

    let dt_utc = dt_local.naive_utc().and_utc();

    Ok(dt_utc)
}
#[derive(Debug, Deserialize)]
struct UNHWxEntry {
    #[serde(rename="Datetime")]
    #[serde(deserialize_with="deserialize_unh_dt")]
    dt: DateTime<Utc>,

    // #[serde(rename="RecNbr")]
    // record_num: usize,

    #[serde(rename="WS_mph_Avg")]
    wind_speed: f32,

    // #[serde(rename="PAR_Den_Avg")]
    // photo_rad: f32,

    // #[serde(rename="WS_mph_S_WVT")]
    // wind_speed_dev: f32,

    // #[serde(rename="WindDir_SD1_WVT")]
    // wind_dir_dev: f32,

    #[serde(rename="AirTF_Avg")]
    temperature_2m: f32,

    #[serde(rename="Rain_in_Tot")]
    rain: f32,

    #[serde(rename="RH")]
    relative_humidity: f32,

    #[serde(rename="WindDir_D1_WVT")]
    wind_dir: f32,
}

impl UNHWxEntry {
    fn to_wx_entry(self) -> WxEntry {
        let unh_station = Station {
            name: "UNH".into(),
            altitude: 28.0, //meters
            coords: (43.1348, -70.9358)
        };

        let mut layers = HashMap::new();

        layers.insert(Layer::NearSurface, WxEntryLayer { 
            layer: Layer::NearSurface, 
            height_agl: Some(6.0), 
            height_msl: Some(28.0), 
            temperature: Some(self.temperature_2m), 
            dewpoint: Some(rh_to_dewpoint(self.temperature_2m, self.relative_humidity)), 
            pressure: None, 
            wind_direction: Direction::from_degrees(self.wind_dir as u16).ok(), 
            wind_speed: Some(self.wind_speed), 
            visibility: None,

            relative_humidity: None,
            slp: None,
            wind_chill: None,
            heat_index: None,
            apparent_temp: None,
        });
    
        let mut entry = WxEntry { 
            date_time: self.dt, 
            station: unh_station, 
            layers, 
            cape: None, 
            skycover: None, 
            wx_codes: None, 
            raw_metar: None, 
            precip_today: None, 
            precip: Some(Precip {
                rain: self.rain,
                snow: 0.,
                unknown: 0.,
            }), 
            precip_probability: None,
            altimeter: None,
            
            wx: None,
            best_slp: None,
        };

        entry.fill_in_calculated_values();

        return entry

    }
}