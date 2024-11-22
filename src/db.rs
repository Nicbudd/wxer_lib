use std::{collections::BTreeMap, fs::File, sync::Arc};
use futures::lock::Mutex;
use chrono::{DateTime, Duration, Utc};
use super::*;

pub type StationData = BTreeMap<DateTime<Utc>, WxEntryStruct>;
pub type StationDatabase = Arc<Mutex<StationDatabaseInternal>>;

#[derive(Debug, Clone)]
pub struct StationDatabaseInternal {
    pub station: Station,
    pub data: StationData
}

pub fn new_station_db(station: Station) -> StationDatabase {
    return Arc::new(Mutex::from(StationDatabaseInternal {
        station: station,
        data: BTreeMap::new()
    }))
}

pub trait DatabaseFuncs { // not sure what to call this
    #[allow(async_fn_in_trait)]
    async fn add(&self, child: StationData, replace: bool);
    #[allow(async_fn_in_trait)]
    async fn export(&self, name: &str, date: DateTime<Utc>) -> Result<()>;
    #[allow(async_fn_in_trait)]
    async fn trim(&self);
    #[allow(async_fn_in_trait)]
    async fn full_update(&self, child: Result<StationData>, replace: bool, name: &str, date: DateTime<Utc>) -> Result<()>;
}


impl DatabaseFuncs for StationDatabase {
    async fn add(&self, child: StationData, replace: bool) {
        let mut db = self.lock().await;
        for (k , v) in child {
            if replace || !db.data.contains_key(&k) {
                db.data.insert(k, v);
            }
        }
    }
    
    async fn export(&self, name: &str, date: DateTime<Utc>) -> Result<()> {
        let file_path: String = format!("data/{}_{}.json", name, date.format("%Y-%m-%d"));
        let mut write_tree: StationData = BTreeMap::new();
        
        let db = self.lock().await;
        for (dt, entry) in db.data.iter() {
            if dt.date_naive() == date.date_naive() {
                write_tree.insert(*dt,  entry.clone());
            }
        }
        drop(db);
    
        let file = File::create(&file_path)?;
        serde_json::ser::to_writer(file, &write_tree)?;
    
        Ok(())
    }

    async fn trim(&self) {
        let now = Utc::now();
    
        let mut db = self.lock().await;
        let keys: Vec<_> = db.data.keys().cloned().collect();
        for e in keys {
            if e < now - Duration::days(2) {
                db.data.remove(&e);
            }
        }
    }

    async fn full_update(&self, child: Result<StationData>, replace: bool, name: &str, date: DateTime<Utc>) -> Result<()> {
        let one_day = Duration::days(1);

        self.add(child.unwrap_or_default(), replace).await;
        self.export(name, date).await?;
        self.export(name, date - one_day).await?;
        self.trim().await;
        Ok(())
    }
}