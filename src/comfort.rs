//! "Comfort Index" based on my personal preferences. May not closely match
//! anyone else's wishes and desires.

use serde::{Deserialize, Serialize};
use strum::Display;

use crate::{
    FractionalUnit::Percent, Intensity, TemperatureUnit::Fahrenheit, Unit, WxEntry, WxEntryLayer,
    WxEntryStruct,
};

/// returns the comfort index and a factor representing the worst condition faced
pub fn comfort_index(wx: WxEntryStruct) -> Option<(u8, Factor)> {
    let temp = wx
        .surface()
        .and_then(|x| x.temperature())
        .map(|x| get_from_table(&x.value_in(Fahrenheit), &TEMPERATURE_FACTORS));

    let cloud = wx.skycover().map(|x| {
        let oktas = x.oktas() as f32;
        get_from_table(&oktas, &CLOUD_COVER_DAY_FACTORS)
    });

    let rain = wx.wx().map(|w| {
        // fuck freezing raining
        if w.freezing && !matches!(w.rain, Intensity::None | Intensity::Nearby) {
            return 0;
        }

        if w.fog {
            return 9;
        }

        match w.rain {
            Intensity::None | Intensity::Nearby => 10,
            Intensity::VeryLight => 7,
            Intensity::Light => 6,
            Intensity::Medium => 4,
            Intensity::Heavy => 5,
        }
    });

    let lightning_modifier = wx
        .wx()
        .map(|w| if w.thunderstorm { 5 } else { 0 })
        .unwrap_or(0);

    let snow_modifier = wx
        .wx()
        .map(|w| {
            if matches!(w.snow, Intensity::None | Intensity::Nearby) {
                return 0;
            }

            if w.thunderstorm {
                return 10;
            }

            if w.squalls {
                return 5;
            }

            match w.snow {
                Intensity::VeryLight => 1,
                Intensity::Light => 2,
                Intensity::Medium => 3,
                Intensity::Heavy => 5,
                Intensity::None | Intensity::Nearby => 0,
            }
        })
        .unwrap_or(0);

    let tornado_modifier = wx
        .wx()
        .map(|w| {
            if w.funnel_cloud == Intensity::None {
                0
            } else {
                10
            }
        })
        .unwrap_or(0);

    let heat_index = wx
        .surface()
        .and_then(|x| x.heat_index())
        .map(|x| get_from_table(&x.value_in(Fahrenheit), &HEAT_INDEX_FACTORS));

    let wind_chill = wx
        .surface()
        .and_then(|x| x.wind_chill())
        .map(|x| get_from_table(&x.value_in(Fahrenheit), &WIND_CHILL_FACTORS));

    let rh = wx
        .surface()
        .and_then(|x| x.relative_humidity())
        .map(|x| get_from_table(&x.value_in(Percent), &RELATIVE_HUMIDITY_FACTORS));

    let dew_point = wx
        .surface()
        .and_then(|x| x.dewpoint())
        .map(|x| get_from_table(&x.value_in(Fahrenheit), &DEWPOINT_FACTORS));

    let factors = [
        (temp, Factor::Temperature),
        (cloud, Factor::CloudCover),
        (heat_index, Factor::HeatIndex),
        (wind_chill, Factor::WindChill),
        (rain, Factor::Rain),
        (rh, Factor::DryAir),
        (dew_point, Factor::Humidity),
    ];

    let (index, worst_factor) = factors
        .iter()
        .min_by(|a, b| a.0.unwrap_or(10).cmp(&b.0.unwrap_or(10)))?;

    let mut index = (*index)?;
    index += lightning_modifier + snow_modifier + tornado_modifier;
    index = index.min(10);

    Some((index, *worst_factor))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Display)]
pub enum Factor {
    Temperature,
    #[strum(serialize = "Cloud Cover")]
    CloudCover,
    #[strum(serialize = "Rain")]
    Rain,
    #[strum(serialize = "Heat Index")]
    HeatIndex,
    #[strum(serialize = "Wind Chill")]
    WindChill,
    Humidity,
    #[strum(serialize = "Dry Air")]
    DryAir,
}

pub fn get_from_table(value: &f32, table: &[(f32, u8)]) -> u8 {
    for (max, r) in table {
        if value >= max {
            return *r;
        }
    }
    table.last().unwrap().1
}

pub const TEMPERATURE_FACTORS: [(f32, u8); 14] = [
    (105., 0),
    (95., 2),
    (90., 4),
    (85., 5),
    (77., 8),
    (65., 10),
    (55., 9),
    (45., 7),
    (38., 4),
    (35., 3),
    (27., 4),
    (20., 2),
    (10., 1),
    (f32::MIN, 0),
];

pub const CLOUD_COVER_NIGHT_FACTORS: [(f32, u8); 4] = [(7., 8), (5., 9), (1., 10), (f32::MIN, 10)];
pub const CLOUD_COVER_DAY_FACTORS: [(f32, u8); 4] = [(7., 8), (5., 9), (1., 10), (f32::MIN, 9)];

pub const HEAT_INDEX_FACTORS: [(f32, u8); 6] = [
    (105., 0),
    (100., 1),
    (95., 3),
    (85., 5),
    (80., 8),
    (f32::MIN, 10),
];

pub const WIND_CHILL_FACTORS: [(f32, u8); 8] = [
    (65., 10),
    (45., 8),
    (35., 5),
    (27., 4),
    (22., 3),
    (15., 2),
    (5., 1),
    (f32::MIN, 0),
];

pub const RELATIVE_HUMIDITY_FACTORS: [(f32, u8); 3] = [(20., 10), (10., 5), (0., 2)];

pub const DEWPOINT_FACTORS: [(f32, u8); 6] = [
    (75., 2),
    (70., 5),
    (65., 8),
    (20., 10),
    (0., 8),
    (f32::MIN, 3),
];
