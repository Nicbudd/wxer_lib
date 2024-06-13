use std::{collections::BTreeMap, fs::File};
use chrono::{DateTime, Duration, Utc};
use super::*;

pub type StationDatabase = BTreeMap<DateTime<Utc>, WxEntry>;

pub async fn export_db(name: &str, date: DateTime<Utc>, db: &mut BTreeMap<DateTime<Utc>, WxEntry>) -> Result<()> {

    let file_path: String = format!("data/{}_{}.json", name, date.format("%Y-%m-%d"));

    let mut write_vec: BTreeMap<&DateTime<Utc>, &mut WxEntry> = BTreeMap::new();

    for (dt, entry) in db {
        if dt.date_naive() == date.date_naive() {
            write_vec.insert(dt,  entry);
        }
    }

    let file = File::create(&file_path)?;

    serde_json::ser::to_writer(file, &write_vec)?;


    Ok(())
}

pub async fn trim_db(db: &mut BTreeMap<DateTime<Utc>, WxEntry>) {
    let now = Utc::now();

    let keys: Vec<_> = db.keys().cloned().collect();

    for e in keys {
        if e < now - Duration::days(2) {
            db.remove(&e);
        }
    }
}