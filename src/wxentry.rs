use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use std::{collections::HashMap, f32::consts::PI, fmt};

use crate::formulae::*;
use crate::units::*;

mod components;
pub use components::Layer::*;
pub use components::*;

mod entry_struct;
pub use entry_struct::*;

mod entry_deser;
pub use entry_deser::*;

mod wxall;
pub use wxall::*;

mod hashmap;
pub use hashmap::*;

pub trait WxEntry<'a, L: WxEntryLayer>
where
    Self: fmt::Debug,
{
    // REQUIRED FIELDS ---------------------------------------------------------
    fn date_time(&self) -> DateTime<Utc>;
    fn station(&self) -> &'static Station;
    fn layer(&'a self, layer: Layer) -> Option<L>;
    fn layers(&self) -> Vec<Layer>;
    // fn new(station: &Station) -> Self;

    // OPTIONAL FIELDS ---------------------------------------------------------
    fn skycover(&self) -> Option<SkyCoverage> {
        None
    }
    fn wx_codes(&self) -> Option<Vec<String>> {
        None
    }
    fn raw_metar(&self) -> Option<String> {
        None
    }
    fn precip_today(&self) -> Option<Precip> {
        None
    }
    fn precip_probability(&self) -> Option<Fraction> {
        None
    }
    fn precip(&self) -> Option<Precip> {
        None
    }
    fn altimeter(&self) -> Option<Pressure> {
        None
    }
    fn cape(&self) -> Option<SpecEnergy> {
        None
    }

    // CALCULATED FIELDS -------------------------------------------------------

    fn wx(&self) -> Option<Wx> {
        let mut wx = Wx::none();
        let codes = self.wx_codes().clone()?; // stupid, why do I have to clone here
        for code in codes {
            wx = wx.combine(Wx::parse_code(&code));
        }
        Some(wx)
    }

    fn latitude(&self) -> f32 {
        self.station().coords.latitude
    }

    fn best_slp(&'a self) -> Option<Pressure> {
        let option_1 = { self.layer(SeaLevel).and_then(|x| x.pressure()) };
        let option_2 = { self.layer(NearSurface).and_then(|x| x.slp()) };
        let option_3 = { self.layer(Indoor).and_then(|x| x.slp()) };
        let option_4 = { self.mslp_from_altimeter() };

        option_1.or(option_2).or(option_3).or(option_4)
    }

    fn date_time_local(&self) -> DateTime<Tz> {
        self.date_time().with_timezone(&self.station().time_zone)
    }

    // ACCESSOR FIELDS ---------------------------------------------------------

    fn sealevel(&'a self) -> Option<L> {
        self.layer(SeaLevel)
    }

    fn surface(&'a self) -> Option<L> {
        self.layer(NearSurface)
    }

    fn indoor(&'a self) -> Option<L> {
        self.layer(Indoor)
    }

    fn to_struct(&'a self) -> Result<WxEntryStruct> {
        let mut layers = HashMap::new();

        for layer in self.layers() {
            let layer = self
                .layer(layer)
                .context("Layer in layers() was not contained in layer(layer).")?;

            let l = WxEntryLayerStruct {
                layer: layer.layer(),
                station: self.station(),
                temperature: layer.temperature(),
                pressure: layer.pressure(),
                dewpoint: layer.dewpoint(),
                visibility: layer.visibility(),
                wind: layer.wind(),
                height_msl: layer.height_msl(),
            };

            layers.insert(layer.layer(), l);
        }

        Ok(WxEntryStruct {
            altimeter: self.altimeter(),
            cape: self.cape(),
            date_time: self.date_time(),
            station: self.station(),
            layers,
            skycover: self.skycover(),
            wx_codes: self.wx_codes(),
            raw_metar: self.raw_metar(),
            precip_today: self.precip_today(),
            precip_probability: self.precip_probability(),
            precip: self.precip(),
        })
    }

    // FOR USE BY IMPLEMENTORS -------------------------------------------------

    fn station_pressure_from_altimeter(&self) -> Option<Pressure> {
        Some(altimeter_to_station(
            self.altimeter()?,
            self.station().altitude,
        ))
    }

    fn mslp_from_altimeter(&'a self) -> Option<Pressure> {
        let surface = self.layer(NearSurface)?;
        Some(altimeter_to_slp(
            self.altimeter()?,
            self.station().altitude,
            surface.temperature()?,
        ))
    }
}

pub trait WxEntryLayer {
    fn layer(&self) -> Layer;
    fn station(&self) -> &'static Station;

    // OPTIONAL FIELDS ---------------------------------------------------------

    fn temperature(&self) -> Option<Temperature> {
        None
    }
    fn pressure(&self) -> Option<Pressure> {
        None
    }
    fn visibility(&self) -> Option<Distance> {
        None
    }
    fn wind(&self) -> Option<Wind> {
        None
    }

    // QUASI-CALCULATED FIELDS -------------------------------------------------
    // completing one of these fields will complete the others

    fn dewpoint(&self) -> Option<Temperature> {
        self.dewpoint_from_rh()
    }
    fn relative_humidity(&self) -> Option<Fraction> {
        self.rh_from_dewpoint()
    }

    // CALCULATED FIELDS -------------------------------------------------------

    fn height_agl(&self) -> Option<Altitude> {
        self.layer().height_agl(self.station().altitude)
    }

    fn height_msl(&self) -> Option<Altitude> {
        self.height_agl().map(|x| x - self.station().altitude)
    }

    fn slp(&self) -> Option<Pressure> {
        let p = self.pressure()?.value_in(Mbar);
        let t = self.temperature()?.value_in(Celsius);
        let h = self.height_msl().map(|x| x.value_in(Meter))?;

        // http://www.wind101.net/sea-level-pressure-advanced/sea-level-pressure-advanced.html
        let phi = self.station().coords.latitude * PI / 180.0;
        let b = 1013.25; //(average baro pressure of a column)
        let k_upper = 18400.; // meters apparently
        let alpha = 0.0037; // coefficient of thermal expansion of air
        let k_lower = 0.0026; // based on figure of earth
        let r = 6367324.; // radius of earth

        let lapse_rate = 0.005; // 0.5C/100m

        let column_temp = t + (lapse_rate * h) / 2.; // take the average of the temperature
        let e = 10f32.powf(7.5 * column_temp / (237.3 + column_temp)) * 6.1078;

        let term1 = 1. + (alpha * column_temp); // correction for column temp
        let term2 = 1. / (1. - (0.378 * (e / b))); // correction for humidity
        let term3 = 1. / (1. - (k_lower * (2. * phi).cos())); // correction for obliquity of earth
        let term4 = 1. + (h / r); // correction for gravity

        let correction = h / (k_upper * term1 * term2 * term3 * term4);

        let mslp = 10f32.powf(p.log10() + correction);

        Some(Pressure::new(mslp, Mbar))
    }

    // None - Incomplete Data
    // Some(true) - wind chill is within valid temp & wind range
    // Some(false) - wind chill is outside valid temp and wind range
    fn wind_chill_valid(&self) -> Option<bool> {
        let t = self.temperature()?.value_in(Fahrenheit);
        if t < 50. {
            let mph = self.wind_speed()?.value_in(Mph);
            Some(mph > 3.)
        } else {
            Some(false)
        }
    }

    fn wind_chill(&self) -> Option<Temperature> {
        let mph = self.wind_speed()?.value_in(Mph);
        let t = self.temperature()?.value_in(Fahrenheit);

        if self.wind_chill_valid() == Some(true) {
            let v_016 = mph.powf(0.16);
            let wc_f = 35.74 + 0.6215 * t - 35.75 * v_016 + 0.4275 * t * v_016;
            Some(Temperature::new(wc_f, Fahrenheit))
        } else {
            None
        }
    }

    fn wind_speed(&self) -> Option<Speed> {
        Some(self.wind()?.speed)
    }

    fn wind_direction(&self) -> Option<Direction> {
        self.wind()?.direction
    }

    // None - Incomplete Data
    // Some(true) - heat index is within valid temp & humidity range
    // Some(false) - heat index is outside valid temp & humidity range
    fn heat_index_valid(&self) -> Option<bool> {
        let t = self.temperature()?.value_in(Fahrenheit);

        if t > 80. {
            let rh = self.relative_humidity()?.value_in(Percent);
            Some(rh > 40.)
        } else {
            Some(false)
        }
    }

    // from Wikipedia: https://en.wikipedia.org/wiki/Heat_index
    fn heat_index(&self) -> Option<Temperature> {
        let t = self.temperature()?.value_in(Fahrenheit) as f64;
        let rh = self.relative_humidity()?.value_in(Percent) as f64;

        if self.heat_index_valid() == Some(true) {
            const C: [f64; 10] = [
                0.0,
                -42.379,
                2.04901523,
                10.14333127,
                -0.22475541,
                -0.00683783,
                -0.05481717,
                0.00122874,
                0.00085282,
                -0.00000199,
            ];
            let hi_f = (C[1])
                + (C[2] * t)
                + (C[3] * rh)
                + (C[4] * t * rh)
                + (C[5] * t * t)
                + (C[6] * rh * rh)
                + (C[7] * t * t * rh)
                + (C[8] * t * rh * rh)
                + (C[9] * t * t * rh * rh);
            Some(Temperature::new(hi_f as f32, Fahrenheit))
        } else {
            None
        }
    }

    fn apparent_temp(&self) -> Option<Temperature> {
        // dbg!(self.heat_index_valid(), self.wind_chill_valid());
        let _ = self.temperature()?;

        match (self.heat_index_valid(), self.wind_chill_valid()) {
            (Some(true), _) => self.heat_index(), // if the heat index is valid, use it
            (_, Some(true)) => self.wind_chill(), // if the wind chill is valid, use it
            (Some(false), Some(false)) => self.temperature(), // if we're outside the range of both, then we can just use temp_2m
            (None, _) | (_, None) => None, // if neither are valid and we're missing data, then we can't provide a valid index
        }
    }

    fn theta_e(&self, altimeter: Option<Pressure>) -> Option<Temperature> {
        let pressure;
        if let Some(p) = self.pressure() {
            pressure = p;
        } else if let Some(alt_pres) = altimeter {
            pressure = altimeter_to_station(alt_pres, self.height_msl()?)
        } else {
            return None;
        }

        Some(theta_e(self.temperature()?, self.dewpoint()?, pressure))
    }

    // QUASI-CALCULATED IMPLEMENTATIONS ----------------------------------------

    fn dewpoint_from_rh(&self) -> Option<Temperature> {
        Some(dewpoint_from_rh(
            self.temperature()?,
            self.relative_humidity()?,
        ))
    }

    fn rh_from_dewpoint(&self) -> Option<Fraction> {
        // in percentage
        Some(rh_from_dewpoint(self.temperature()?, self.dewpoint()?))
    }

    fn wind_from_speed_and_direction(&self) -> Option<Wind> {
        Some(Wind {
            direction: self.wind_direction(),
            speed: self.wind_speed()?,
        })
    }

    fn to_struct(&self) -> WxEntryLayerStruct {
        WxEntryLayerStruct {
            dewpoint: self.dewpoint(),
            layer: self.layer(),
            station: self.station(),
            temperature: self.temperature(),
            pressure: self.pressure(),
            visibility: self.visibility(),
            wind: self.wind(),
            height_msl: self.height_msl(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono_tz::US::Eastern;

    use crate::*;

    struct TestLayer {
        layer: Layer,
        station: &'static Station,
        temperature: Option<Temperature>,
        wind_speed: Option<Speed>,
        dewpoint: Option<Temperature>,
    }

    impl WxEntryLayer for TestLayer {
        fn layer(&self) -> Layer {
            self.layer
        }
        #[allow(refining_impl_trait)]
        fn station(&self) -> &'static Station {
            self.station
        }
    }

    fn default_station() -> &'static Station {
        let b = Box::new(Station {
            name: String::from("Test"),
            altitude: Altitude::new(0., Meter),
            coords: (0., 0.).into(),
            time_zone: Eastern,
        });
        Box::leak(b)
    }

    fn float_within_one_decimal(val: f32, cmp: f32) -> bool {
        if val < (cmp + 0.1) && val > (cmp - 0.1) {
            true
        } else {
            println!("{val}");
            false
        }
    }

    #[test]
    fn test_apparent_temp() {
        let mut e = TestLayer {
            layer: NearSurface,
            station: default_station(),
            dewpoint: None,
            temperature: None,
            wind_speed: None,
        };

        e.temperature = Some(Temperature::new(51., Fahrenheit));
        assert_eq!(e.apparent_temp(), Some(Temperature::new(51., Fahrenheit)));

        e.temperature = Some(Temperature::new(49., Fahrenheit));
        assert_eq!(e.apparent_temp(), None);

        e.wind_speed = Some(Speed::new(2., Mph));
        assert_eq!(e.apparent_temp(), Some(Temperature::new(49., Fahrenheit)));

        e.temperature = Some(Temperature::new(79., Fahrenheit));
        assert_eq!(e.apparent_temp(), Some(Temperature::new(79., Fahrenheit)));

        e.temperature = Some(Temperature::new(81., Fahrenheit));
        assert_eq!(e.apparent_temp(), None);

        e.dewpoint = Some(Temperature::new(54., Fahrenheit));
        assert_eq!(e.apparent_temp(), Some(Temperature::new(81., Fahrenheit)));

        // heat index tests

        e.temperature = Some(Temperature::new(81., Fahrenheit));
        e.dewpoint = Some(Temperature::new(65., Fahrenheit));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(
            apparent_temp.value_in(Fahrenheit),
            82.8
        ));

        e.temperature = Some(Temperature::new(100., Fahrenheit));
        e.dewpoint = Some(Temperature::new(75., Fahrenheit));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(
            apparent_temp.value_in(Fahrenheit),
            113.7
        ));

        e.temperature = Some(Temperature::new(110., Fahrenheit));
        e.dewpoint = Some(Temperature::new(85., Fahrenheit));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(
            apparent_temp.value_in(Fahrenheit),
            146.1
        ));

        // wind chill tests

        e.temperature = Some(Temperature::new(32., Fahrenheit));
        e.wind_speed = Some(Speed::new(10., Mph));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(
            apparent_temp.value_in(Fahrenheit),
            23.0
        ));

        e.temperature = Some(Temperature::new(49., Fahrenheit));
        e.wind_speed = Some(Speed::new(3., Mph));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(
            apparent_temp.value_in(Fahrenheit),
            48.1
        ));

        e.temperature = Some(Temperature::new(49., Fahrenheit));
        e.wind_speed = Some(Speed::new(40., Mph));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(
            apparent_temp.value_in(Fahrenheit),
            38.9
        ));

        e.temperature = Some(Temperature::new(-20., Fahrenheit));
        e.wind_speed = Some(Speed::new(3., Mph));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(
            apparent_temp.value_in(Fahrenheit),
            -30.7
        ));

        e.temperature = Some(Temperature::new(-20., Fahrenheit));
        e.wind_speed = Some(Speed::new(40., Mph));
        let apparent_temp = e.apparent_temp().unwrap();
        assert!(float_within_one_decimal(
            apparent_temp.value_in(Fahrenheit),
            -58.4
        ));
    }
}
