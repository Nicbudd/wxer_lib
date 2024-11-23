use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Timelike, Utc};
use anyhow::Result;
use chrono_tz::US::Eastern;
use serde::Deserialize;

use crate::*;
use crate::Layer::*;


// Imports data from my raspberry pi station.
// Code is not online yet, but I plan to open source it.
// For now, this is probably going to see no use from anyone else but myself.

// station URL is going to be something like http://rpi_address:8000
// todo: better name for rpi station
pub async fn import(station_url: &str, date: DateTime<Utc>, station: Arc<Station>) -> Result<db::StationData> {

    // let station_data_url = format!("{station_url}/location.json");

    // use this later
    // let station = reqwest::get(station_data_url)
    //     .await?
    //     .text().await?;

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
        match try_parse_entry(record) {
            Ok((dt, entry)) => {local_db.insert(dt, entry.to_struct()?);},
            Err(e) => {eprintln!("Error parsing entry {i}: {e}");}
        }
    } 

    // dbg!();

    Ok(local_db)
}

fn try_parse_entry(record: Result<RaspPiEntry, csv::Error>) -> Result<(DateTime<Utc>, HashMapWx)> {
    let record: RaspPiEntry = record?;

    let time_string = String::from(record.time.clone()) + "Z";
    let mut dt = time_string.trim().chars().filter(|x| x != &'\0').collect::<String>().parse::<DateTime<Utc>>()?;
    dt = dt - Duration::seconds(dt.second() as i64 + 60); // to account for when the data collection ends

    let station: Station = Station { 
        name: "RPI@ValentineHouse".into(), 
        altitude: Altitude::new(4.8, Meter), 
        coords: (43.13, -70.92).into(), 
        time_zone: Eastern,
    };

    let mut wx = HashMapWx::new(dt, station);

    wx.put(Indoor, Param::Temperature, record.indoor_temp);
    wx.put(NearSurface, Param::Temperature, record.outdoor_temp);
    wx.put(NearSurface, Param::Dewpoint, record.dewpoint);
    wx.put(NearSurface, Param::Pressure, record.raw_pres);
    wx.put(Indoor, Param::Pressure, record.raw_pres);
    
    Ok((dt, wx))
}

#[derive(Debug, Deserialize)]
struct RaspPiEntry {
    time: String,
    indoor_temp: Option<f32>,
    outdoor_temp: Option<f32>,
    // rh: Option<f32>,
    dewpoint: Option<f32>,
    raw_pres: Option<f32>,
}