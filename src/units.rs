// PUBLIC UNIT TRAIT -----------------------------------------------------------

pub trait Unit<T: UnitsType>: where Self: Clone + Copy + Sized + fmt::Display {
    fn new(value: f32, unit: T) -> Self;
    // don't want users accidentally accessing the value without checking the unit
    fn unit(&self) -> T;
    fn convert(&self, unit: T) -> Self;
    fn string_with_unit(&self) -> String;
    fn value_in(&self, unit: T) -> f32;
} 


use core::fmt;

pub use hidden::*;

mod hidden {
    use std::fmt;
    use std::ops::{Add, Div, Mul, Sub};
    use serde::{ser::SerializeStruct, Serialize};
    use strum_macros::Display;
    use anyhow::{Result, bail};
    use super::*;

    // INTERNAL USE UNIT TRAITS  -----------------------------------------------

    trait UnitInternal<T: UnitsType> where Self: Clone + Copy + Sized + fmt::Display {

        fn new(value: f32, unit: T) -> Self;
        fn value(&self) -> f32;
        fn unit(&self) -> T;
        fn convert(&self, unit: T) -> Self {
            if self.unit() == unit { // avoid conversions if the units are the same
                return self.clone()
            } else {
                return Self::convert(&self, unit);
            }
        }

        fn string_with_unit(&self) -> String {
            format!("{:.1} {}", self.value(), UnitInternal::unit(self))
        }
        fn value_in(&self, unit: T) -> f32 { // get the value of a unit in some other unit
            UnitInternal::convert(self, unit).value()
        }
    }

    pub trait UnitsType: Clone + Copy + PartialEq + Eq + fmt::Display + fmt::Debug + Serialize {}

    // PROPORTIONAL UNIT STRUCT ------------------------------------------------
    // helpful for building most units (where the conversion between units are proportional)

    #[derive(Clone, Copy, Debug)]
    pub struct ProportionalUnit<T: Proportional> {
        value: f32,
        unit: T,
    }
    pub trait Proportional: UnitsType {
        fn coefficient(&self) -> f32; // the coefficient that when multiplied by
                           // the value would convert this unit into the 
                           // "default" unit.
    }
    impl<T: Proportional> UnitInternal<T> for ProportionalUnit<T> {
        fn new(value: f32, unit: T) -> Self {
            Self {value, unit}
        }
        fn value(&self) -> f32 {self.value}
        fn unit(&self) -> T {self.unit}

        fn convert(&self, unit: T) -> Self {
            let value_as_default_unit = ProportionalUnit::value(self) * 
                UnitInternal::unit(self).coefficient();

            let value_in_new_unit = value_as_default_unit / unit.coefficient();

            ProportionalUnit {
                unit,
                value: value_in_new_unit
            }
        }
    }

    impl<T: Proportional> fmt::Display for ProportionalUnit<T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", Unit::string_with_unit(self))
        }
    }

    // UNITS -------------------------------------------------------------------

    // WIND ----------------------------------------------------------------
    pub type Speed = ProportionalUnit<SpeedUnit>;

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Display, Serialize)]
    #[allow(unused)]
    pub enum SpeedUnit {
        #[strum(to_string = "mph")]
        Mph, 
        #[strum(to_string = "kph")]
        Kph, 
        #[strum(to_string = "kts")]
        Knots,
        #[strum(to_string = "m/s")]
        Mps,  
    }
    pub use SpeedUnit::*;

    impl UnitsType for SpeedUnit {}
    impl Proportional for SpeedUnit {
        fn coefficient(&self) -> f32 {
            match self {
                Kph => 1.,
                Mph => 1.609344,
                Knots => 1.852,
                Mps => 3.6,
            }
        }
    }

    // PRESSURE ----------------------------------------------------------------
    pub type Pressure = ProportionalUnit<PressureUnit>;

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Display, Serialize)]
    #[allow(unused)]
    pub enum PressureUnit {
        #[strum(to_string = "hPa")]
        HPa, 
        #[strum(to_string = "mb")]
        Mbar, 
        #[strum(to_string = "inHg")]
        InHg,
        #[strum(to_string = "psi")]
        Psi,  
        #[strum(to_string = "atm")]
        Atm,  
    }
    pub use PressureUnit::*;

    impl UnitsType for PressureUnit {}
    impl Proportional for PressureUnit {
        fn coefficient(&self) -> f32 {
            match self {
                HPa => 1.,
                Mbar => 1.,
                Psi => 68.94757,
                Atm => 1013.25,
                InHg => 33.86389,
            }
        }
    }

    // SPECIFIC ENERGY ---------------------------------------------------------
    pub type SpecEnergy = ProportionalUnit<SpecEnergyUnit>;

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Display, Serialize)]
    #[allow(unused)]
    pub enum SpecEnergyUnit {
        #[strum(to_string = "J/kg")]
        Jkg, 
        #[strum(to_string = "m^2/s^2")]
        M2s2, 
    }
    pub use SpecEnergyUnit::*;

    impl UnitsType for SpecEnergyUnit {}
    impl Proportional for SpecEnergyUnit {
        fn coefficient(&self) -> f32 {
            match self {
                Jkg => 1.,
                M2s2 => 1.,
            }
        }
    }

    // DISTANCE ----------------------------------------------------------------
    pub type Distance = ProportionalUnit<DistanceUnit>;
    pub type Altitude = ProportionalUnit<DistanceUnit>;

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Display, Serialize)]
    #[allow(unused)]
    pub enum DistanceUnit {
        #[strum(to_string = "m")]
        Meter, 
        #[strum(to_string = "km")]
        Kilometer, 
        #[strum(to_string = "ft")]
        Feet, 
        #[strum(to_string = "mi")]
        Mile, 
        #[strum(to_string = "nmi")]
        NauticalMile, 
    }
    pub use DistanceUnit::*;

    impl UnitsType for DistanceUnit {}
    impl Proportional for DistanceUnit {
        fn coefficient(&self) -> f32 {
            match self {
                Meter => 1.,
                Kilometer => 1000.,
                Feet => 0.3048,
                Mile => 1609.344,
                NauticalMile => 1852.,
            }
        }
    }

    // PRECIP AMOUNT -----------------------------------------------------------
    pub type PrecipAmount = ProportionalUnit<PrecipUnit>;

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Display, Serialize)]
    #[allow(unused)]
    pub enum PrecipUnit {
        #[strum(to_string = "mm")]
        Mm, 
        #[strum(to_string = "in")]
        Inch, 
        #[strum(to_string = "in")]
        Cm, 
    }
    pub use PrecipUnit::*;

    impl UnitsType for PrecipUnit {}
    impl Proportional for PrecipUnit {
        fn coefficient(&self) -> f32 {
            match self {
                Mm => 1.,
                Inch => 25.4,
                Cm => 2.54,
            }
        }
    }

    // PERCENTAGE -----------------------------------------------------------
    pub type Fraction = ProportionalUnit<FractionalUnit>;

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Display, Serialize)]
    #[allow(unused)]
    pub enum FractionalUnit {
        #[strum(to_string = "%")]
        Percent, 
        #[strum(to_string = "")]
        Decimal, 
        #[strum(to_string = "1/1000")]
        Milli, 
    }
    pub use FractionalUnit::*;

    impl UnitsType for FractionalUnit {}
    impl Proportional for FractionalUnit {
        fn coefficient(&self) -> f32 {
            match self {
                Percent => 0.01,
                Decimal => 1.,
                Milli => 0.001,
            }
        }
    }

    // TEMPERATURE -------------------------------------------------------------
    // Not a proportional unit

    #[derive(Clone, Copy, Debug)]
    pub struct Temperature {
        value: f32,
        unit: TemperatureUnit
    }

    // this is stupid
    impl fmt::Display for Temperature {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", Unit::string_with_unit(self))
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Display, Serialize)]
    #[allow(unused)]
    pub enum TemperatureUnit {
        #[strum(to_string = "°K")]
        Kelvin, 
        #[strum(to_string = "°F")]
        Fahrenheit, 
        #[strum(to_string = "°C")]
        Celsius
    }
    pub use TemperatureUnit::*;

    impl UnitsType for TemperatureUnit {}
    impl UnitInternal<TemperatureUnit> for Temperature {
        fn new(value: f32, unit: TemperatureUnit) -> Self {
            Self {value, unit}
        }
        fn value(&self) -> f32 {self.value}
        fn unit(&self) -> TemperatureUnit {self.unit}

        fn convert(&self, unit: TemperatureUnit) -> Self {
            let value_in_kelvin = match self.unit {
                Kelvin => self.value,
                Celsius => self.value + 273.15,
                Fahrenheit => (self.value + 459.67)*(5./9.)
            };
            let value_in_new_unit = match unit {
                Kelvin => value_in_kelvin,
                Celsius => value_in_kelvin - 273.15,
                Fahrenheit => (value_in_kelvin*(9./5.)) - 459.67
            };
            return Self { 
                value: value_in_new_unit, 
                unit, 
            }
        }
    }

    // DIRECTION ---------------------------------------------------------------
    // does not use the standard unit trait

    #[derive(Clone, Copy, Serialize, derive_more::Display)]
    pub struct Direction(u16); 

    impl Direction {
        fn sanitize_degrees(degrees: u16) -> Result<u16> {
            let degrees = if degrees > 360 {
                bail!("Degrees provided ({degrees}) were not under 360.");
            } else if degrees % 10 != 0 {
                ((degrees + 5) / 10) * 10 // round to nearest 10
            } else {
                degrees
            };

            Ok(degrees % 360)
        }

        pub fn from_degrees(degrees: u16) -> Result<Direction> {
            let corrected_degrees = Direction::sanitize_degrees(degrees)?;
            Ok(Direction(corrected_degrees))
        }

        pub fn cardinal(&self) -> &'static str {
            match self.0 {
                350 | 0 | 10 => "N",
                20 | 30 => "NNE",
                40 | 50 => "NE",
                60 | 70 => "ENE",
                80 | 90 | 100 => "E",
                110 | 120 => "ESE",
                130 | 140 => "SE",
                150 | 160 => "SSE",
                170 | 180 | 190 => "S",
                200 | 210 => "SSW",
                220 | 230 => "SW",
                240 | 250 => "WSW",
                260 | 270 | 280 => "W",
                290 | 300 => "WNW",
                310 | 320 => "NW",            
                330 | 340 => "NNW",
                _ => unreachable!("Direction struct contained {}, which is invalid.", self.0)
            }
        }

        pub fn degrees(&self) -> u16 {
            self.0
        } 
    }

    
    // UNIT TRAIT IMPLEMENTATIONS ----------------------------------------------
    // (boring paperwork to connect the above traits together)

    impl<T: UnitsType, U: UnitInternal<T>> Unit<T> for U where Self: Clone + Copy + Sized {
        // fn value(&self) -> f32 {U::value(&self)}
        fn new(value: f32, unit: T) -> Self {U::new(value, unit)}
        fn unit(&self) -> T {U::unit(&self)}
        fn convert(&self, unit: T) -> Self {U::convert(&self, unit)}
        fn string_with_unit(&self) -> String {U::string_with_unit(&self)}
        fn value_in(&self, unit: T) -> f32 {U::value_in(&self, unit)}
    }

    impl<T: Proportional> Add for ProportionalUnit<T> {
        type Output = Self;
        fn add(self, rhs: Self) -> Self {
            let unit = self.unit;
            let other = Unit::convert(&rhs, unit);
            let value = other.value + self.value;
            Self { value, unit }
        }
    }

    impl<T: Proportional> Sub for ProportionalUnit<T> {
        type Output = Self;
        fn sub(self, rhs: Self) -> Self::Output {
            let unit = self.unit;
            let other = Unit::convert(&rhs, unit);
            let value = other.value - self.value;
            Self { value, unit }
        }
    }

    impl<T: Proportional> Mul<f32> for ProportionalUnit<T> {
        type Output = Self;
        fn mul(self, rhs: f32) -> Self {
            Self { value: self.value*rhs, unit: self.unit }
        }
    }

    impl<T: Proportional> Div<f32> for ProportionalUnit<T> {
        type Output = Self;
        fn div(self, rhs: f32) -> Self {
            Self { value: self.value/rhs, unit: self.unit }
        }
    }

    impl<T: Proportional> PartialEq for ProportionalUnit<T> {
        fn eq(&self, other: &Self) -> bool {
            let other = UnitInternal::convert(other, self.unit);
            self.value == other.value
        }
    }

    impl PartialEq for Temperature {
        fn eq(&self, other: &Self) -> bool {
            let other = UnitInternal::convert(other, self.unit);
            self.value == other.value
        }
    }

    impl<T: Proportional> Serialize for ProportionalUnit<T> {
        fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> where S: serde::Serializer {
            let mut state = serializer.serialize_struct("Unit", 2)?;
            state.serialize_field("value", &self.value)?;
            state.serialize_field("unit", &self.unit)?;
            state.end()
        }
    }
}




// TESTS -----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temperature() {

        let mut t = Temperature::new(68.0, Fahrenheit);

        // keep converting it back and forth between units
        for (_i, u) in [Celsius, Kelvin, Fahrenheit, Kelvin, Celsius, Fahrenheit]
            .iter().enumerate() {
            
            t = t.convert(*u);
            assert!((t.value_in(Fahrenheit) - 68.0).abs() < 0.001);
            assert!((t.value_in(Celsius) - 20.0).abs() < 0.001);
            assert!((t.value_in(Kelvin) - 293.15).abs() < 0.001);
        }

        assert_eq!(t.string_with_unit(), "68.0 °F");
    }

    #[test]
    fn pressure() {

        let mut p = Pressure::new(897., Mbar);

        // keep converting it back and forth between units
        for (_i, u) in [HPa, Mbar, InHg, Psi, Mbar, InHg, HPa, Psi, HPa].iter().enumerate() {
            
            p = p.convert(*u);
            assert!((p.value_in(Mbar) - 897.).abs() < 0.01);
            assert!((p.value_in(HPa) - 897.).abs() < 0.01);
            assert!((p.value_in(InHg) - 26.4883987947).abs() < 0.001);
            assert!((p.value_in(Psi) - 13.0098850743).abs() < 0.001);
            assert!((p.value_in(Atm) - 0.8852701702).abs() < 0.0001);
        }

        assert_eq!(p.string_with_unit(), "897.0 hPa");
    }

    #[test]
    fn speed() {

        let mut s = Speed::new(180., Mph);

        // keep converting it back and forth between units
        for (_i, u) in [Mph, Kph, Knots, Mps, Knots, Kph, Mph].iter().enumerate() {
            
            s = s.convert(*u);
            assert!((s.value_in(Mph) - 180.).abs() < 0.01);
            assert!((s.value_in(Kph) - 289.68192).abs() < 0.01);
            assert!((s.value_in(Knots) - 156.4157235421).abs() < 0.001);
            assert!((s.value_in(Mps) - 80.4672).abs() < 0.001);
        }

        assert_eq!(s.string_with_unit(), "180.0 mph");
    }
    
}