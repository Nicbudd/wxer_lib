use anyhow::Result;

pub mod fetch;
// use fetch::*;

pub mod formulae;
pub use formulae::*;

pub mod db;
// pub use db::*;

pub mod units;
pub use units::*;

pub mod wxentry;
pub use wxentry::*;


// HELPER FUNCTIONS ------------------------------------------------------------

pub fn ignore_none<T, R, F: FnMut(T) -> R>(a: Option<T>, mut f: F) -> Option<R> {
    match a {
        None => None,
        Some(s) => {
            let r = f(s); 
            Some(r)
        }
    }
} 
