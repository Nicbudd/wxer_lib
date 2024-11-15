use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Duration, Timelike, Utc};
use anyhow::Result;
use serde::Deserialize;

use crate::{db::StationData, Layer, Station, WxEntry, WxEntryLayer};


// Imports data from my raspberry pi station.
// Code is not online yet, but I plan to open source it.
// For now, this is probably going to see no use from anyone else but myself.

// station URL is going to be something like http://rpi_address:8000
// todo: better name for rpi station
pub async fn import(station_url: &str, date: DateTime<Utc>) -> Result<StationData> {

    let station_data_url = format!("{station_url}/location.json");
    let station = reqwest::get(station_data_url)
        .await?
        .text().await?;
    let station: Station = serde_json::from_str(&station)?;

    let altitude = station.altitude;

    let url = format!("{station_url}/{}.csv", date.format("%Y-%m-%d").to_string());
    let resp: String = reqwest::get(url)
        .await?
        .text()
        .await?;

    // dbg!(&resp);

    let csv_string = String::from("time,indoor_temp,outdoor_temp,rh,dewpoint,raw_pres,mslp\n") + resp.as_str();

    let mut reader = csv::Reader::from_reader(csv_string.as_bytes());

    let mut local_db = BTreeMap::new();

    for (i, record) in reader.deserialize().enumerate() {
        match try_parse_entry(record, altitude, station.clone()) {
            Ok((dt, entry)) => {local_db.insert(dt, entry);},
            Err(e) => {eprintln!("Error parsing entry {i}: {e}");}
        }
    } 

    // dbg!();

    Ok(local_db)
}

fn try_parse_entry(record: Result<RawStationEntry, csv::Error>, altitude: f32, station: Station) -> Result<(DateTime<Utc>, WxEntry)> {
    let record: RawStationEntry = record?;

    let time_string = String::from(record.time) + "Z";
    let mut dt = time_string.trim().chars().filter(|x| x != &'\0').collect::<String>().parse::<DateTime<Utc>>()?;
    dt = dt - Duration::seconds(dt.second() as i64 + 60); // to account for when the data collection ends

    let indoor = WxEntryLayer {
        layer: Layer::Indoor,
        height_agl: Some(2.0),
        height_msl: Some(altitude),
        temperature: record.indoor_temp,
        dewpoint: None,
        pressure: record.raw_pres,
        wind_direction: None,
        wind_speed: None,
        visibility: None,

        relative_humidity: None,
        slp: None,
        wind_chill: None,
        heat_index: None,
        apparent_temp: None,
        theta_e: None,
    };

    let near_surface = WxEntryLayer {
        layer: Layer::NearSurface,
        height_agl: Some(2.0),
        height_msl: Some(altitude),
        temperature: record.outdoor_temp,
        dewpoint: record.dewpoint,
        pressure: record.raw_pres,
        wind_direction: None,
        wind_speed: None,
        visibility: None,

        relative_humidity: None,
        slp: None,
        wind_chill: None,
        heat_index: None,
        apparent_temp: None,
        theta_e: None,
    };

    let mut layers = HashMap::new();

    layers.insert(Layer::Indoor, indoor);
    layers.insert(Layer::NearSurface, near_surface);

    let mut entry: WxEntry = WxEntry {
        date_time: dt,
        station,

        layers,
        
        cape: None,
        skycover: None,
        raw_metar: None,
        precip_today: None,
        precip: None,
        precip_probability: None,
        wx: None,
        wx_codes: None,
        altimeter: None,

        best_slp: None,
    };
    
    entry.fill_in_calculated_values();

    Ok((dt, entry))
}

#[derive(Debug, Deserialize)]
struct RawStationEntry {
    time: String,
    indoor_temp: Option<f32>,
    outdoor_temp: Option<f32>,
    // rh: Option<f32>,
    dewpoint: Option<f32>,
    raw_pres: Option<f32>,
    // mslp: Option<f32>,
}
