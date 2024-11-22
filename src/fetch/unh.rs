use std::collections::BTreeMap;

use crate::units::*; 
use crate::*;
use crate::db::StationData;

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
        let entry: UNHData = entry_result?;

        db.insert(entry.date_time(), entry.to_struct()?);
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
struct UNHData {
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

impl<'a> WxEntry<'a, &'a UNHData> for UNHData {
    fn date_time(&self) -> DateTime<Utc> {self.dt}
    fn station(&self) -> Station {
        Station {
            name: "UNH".into(),
            altitude: Altitude::new(28.0, Meter), //meters
            coords: (43.1348, -70.9358)
        }
    }
    fn layer(&self, layer: Layer) -> Option<&UNHData> {
        if layer == Layer::NearSurface {
            Some(self)
        } else {
            None
        }
    }
    fn layers(&self) -> Vec<Layer> {vec![Layer::NearSurface]}

    fn precip_today(&self) -> Option<Precip> {
        Some(Precip { 
            unknown: PrecipAmount::new(0., Inch), 
            rain: PrecipAmount::new(self.rain, Inch), 
            snow: PrecipAmount::new(0., Inch) 
        })
    }
} 

impl<'a> WxEntryLayer for &'a UNHData {
    fn layer(&self) -> Layer {Layer::NearSurface}
    fn station(&self) -> Station {
        Station {
            name: "UNH".into(),
            altitude: Altitude::new(28.0, Meter), //meters
            coords: (43.1348, -70.9358)
        }
    }

    fn temperature(&self) -> Option<Temperature> {
        Some(Temperature::new(self.temperature_2m, Fahrenheit))
    }
    fn relative_humidity(&self) -> Option<Fraction> {
        Some(Fraction::new(self.relative_humidity, Percent))
    } 
    fn wind(&self) -> Option<Wind> {
        Some(Wind {
            direction: Direction::from_degrees(self.wind_dir as u16).ok()?,
            speed: Speed::new(self.wind_speed, Mph)
        })
    }
}