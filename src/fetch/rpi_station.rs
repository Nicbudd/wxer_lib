use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Duration, Timelike, Utc};
use anyhow::Result;
use serde::Deserialize;

use crate::{Layer, Station, WxEntry, WxEntryLayer};


// Imports data from my raspberry pi station.
// Code is not online yet, but I plan to open source it.
// For now, this is probably going to see no use from anyone else but myself.

// station URL is going to be something like http://rpi_address:8000
// todo: better name for rpi station
pub async fn import(
    station_url: String, date: DateTime<Utc>, 
    local_db: &mut BTreeMap<DateTime<Utc>, WxEntry>
) -> Result<()> {

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

    let csv_string = String::from("time,indoor_temp,outdoor_temp,rh,dewpoint,raw_pres,mslp\n") + resp.as_str();

    let mut reader = csv::Reader::from_reader(csv_string.as_bytes());

    for record in reader.deserialize() {
        let record: RawStationEntry = record?;

        let time_string = String::from(record.time) + "Z";
        let mut dt = time_string.parse::<DateTime<Utc>>()?;
        dt = dt - Duration::seconds(dt.second() as i64 + 60); // to account for when the data collection ends

        if !local_db.contains_key(&dt) {

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
            };

            let mut layers = HashMap::new();

            layers.insert(Layer::Indoor, indoor);
            layers.insert(Layer::NearSurface, near_surface);

            let entry: WxEntry = WxEntry {
                date_time: dt,
                station: station.clone(),

                layers,
                
                cape: None,
                skycover: None,
                raw_metar: None,
                precip_today: None,
                precip: None,
                precip_probability: None,
                present_wx: None
            };
    
            local_db.insert(dt, entry);
        }
    } 

    Ok(())
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
