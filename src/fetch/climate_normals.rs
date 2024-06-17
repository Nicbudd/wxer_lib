use std::collections::BTreeMap;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};

#[derive(Debug, Deserialize)]
struct NCEIClimateNormalsEntry {
    #[serde(rename="month")]
    month: String,

    #[serde(rename="day")]
    day: String,

    #[serde(rename="DLY-TAVG-NORMAL")]
    avg_temp: String,

    #[serde(rename="DLY-TMAX-NORMAL")]
    max_temp: String,

    #[serde(rename="DLY-TMIN-NORMAL")]
    min_temp: String,

    // #[serde(rename="ELEVATION")]
    // elevation: String,

    // #[serde(rename="LATITUDE")]
    // latitude: String,

    // #[serde(rename="LONGITUDE")]
    // longitude: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct ClimateNormals {
    pub min_temp: f32,
    pub avg_temp: f32,
    pub max_temp: f32,
}


//https://www.ncei.noaa.gov/data/normals-daily/1991-2020/doc/Normals_DLY_Documentation_1991-2020.pdf

pub async fn import(
    ncei_station: &str,
    temp_db: &mut BTreeMap<NaiveDate, ClimateNormals>,
) -> Result<()> {

    let url = format!("https://www.ncei.noaa.gov/data/normals-daily/1991-2020/access/{ncei_station}.csv");

    // dbg!(&url);
    let text = reqwest::get(&url).await?.text().await?;

    // std::fs::write("csv.text", text.as_bytes()).unwrap();

    // dbg!(&text);

    let mut rdr = csv::ReaderBuilder::new()
                                // .has_headers(false)
                                // .double_quote(false)
                                .from_reader(text.as_bytes());

    for entry_result in rdr.deserialize() {
        let entry: NCEIClimateNormalsEntry = entry_result?;

        // dbg!(&entry);

        let date = NaiveDate::from_ymd_opt(2000, entry.month.parse()?, entry.day.parse()?)
            .ok_or(anyhow!("Failed create date from 2000/{}/{}!", entry.month, entry.day))?;

        temp_db.insert(date, ClimateNormals { 
            min_temp: entry.min_temp.trim().parse()?, 
            avg_temp: entry.avg_temp.trim().parse()?, 
            max_temp: entry.max_temp.trim().parse()?, 
        });
        
    }


    Ok(())
}