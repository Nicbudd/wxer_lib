/*
You must include a link next to any location, Open-Meteo data are displayed like:

<a href="https://open-meteo.com/">Weather data by Open-Meteo.com</a>

*/

use std::collections::{BTreeMap, HashMap};

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// todo: convert to WxEntry

#[allow(dead_code)]
pub async fn import_model_data(
    coords: (f32, f32),
    data_type: DataType,
    model: WeatherModel,
    forecast_days: u8,
) -> Result<ModelDataCollection> {
    let url = format!("https://api.open-meteo.com/v1/forecast?latitude={:.2}&longitude={:.2}&hourly={}&models={}&temperature_unit=fahrenheit&windspeed_unit=mph&precipitation_unit=inch&forecast_days={}", coords.0, coords.1, data_type.to_str(), model.to_str(), forecast_days);

    //dbg!(&url);

    let resp: String = reqwest::get(url).await?.text().await?;

    //dbg!(&resp);

    let resp: OpenMeteoResponse = serde_json::from_str(&resp)?;

    let times = resp
        .hourly
        .get("time")
        .ok_or(anyhow!("Times did not exist in open-meteo response."))?;
    let datas = resp
        .hourly
        .get(data_type.to_str())
        .ok_or(anyhow!("Times did not exist in open-meteo response."))?;

    let mut structured_data = ModelDataCollection {
        model,
        data_type,
        data: BTreeMap::new(),
    };

    for i in 0..times.len() {
        let t = times.get(i).unwrap();
        let d = datas.get(i).unwrap();

        if let (Value::String(time), Value::Number(data)) = (t, d) {
            let time_string = String::from(time) + ":00Z";
            let dt = time_string.parse::<DateTime<Utc>>()?;

            let dat = data
                .as_f64()
                .ok_or(anyhow!("Number was not able to be formatted into a f64"))?;

            structured_data.data.insert(dt, dat);
        } else {
            return Err(anyhow!("The type of data from open-meteo is wrong"));
        }
    }

    Ok(structured_data)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelDataEntry {
    pub model: WeatherModel,
    pub data_type: DataType,
    pub data: BTreeMap<DateTime<Utc>, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Temperature2m,
    Dewpoint2m,
    ApparentTemperature,
    Cape,
}

impl DataType {
    pub fn to_str(&self) -> &str {
        match self {
            DataType::Temperature2m => "temperature_2m",
            DataType::Dewpoint2m => "dewpoint_2m",
            DataType::ApparentTemperature => "apparent_temperature",
            DataType::Cape => "cape",
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelRun {
    model: WeatherModel,
    date: DateTime<Utc>,
}

#[derive(Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherModel {
    BestMatch,
    GFSSeamless,
    EcmwfIFS,
}

impl WeatherModel {
    pub fn to_str(&self) -> &str {
        match self {
            WeatherModel::BestMatch => "best_match",
            WeatherModel::GFSSeamless => "gfs_seamless",
            WeatherModel::EcmwfIFS => "ecmwf_ifs04",
        }
    }
}

#[derive(Debug)]
pub struct ModelDataCollection {
    pub model: WeatherModel,
    pub data_type: DataType,
    pub data: BTreeMap<DateTime<Utc>, f64>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct OpenMeteoResponse {
    latitude: f32,
    longitude: f32,
    //generationtime_ms: f32,
    utc_offset_seconds: i32,
    //timezone: String,
    timezone_abbreviation: String,
    //elevation: f32,
    //hourly_units: HashMap<String, String>,
    hourly: HashMap<String, Vec<Value>>,
}
