use std::fmt;

// PUBLIC UNIT TRAIT -----------------------------------------------------------

// #[allow(private_bounds)]
pub trait Unit<T> where 
    Self: Clone + Copy + Sized,
    T: Clone + Copy + PartialEq + Eq + fmt::Display + fmt::Debug {

    fn new(value: f32, unit: T) -> Self;
    // don't want users accidentally accessing the value without checking the unit
    fn unit(&self) -> T;
    fn convert(&self, unit: T) -> Self;
    fn string_with_unit(&self) -> String;
    fn value_in(&self, unit: T) -> f32;
} 


pub use hidden::*;

mod hidden {
    use strum_macros::Display;
    use std::fmt;
    use super::*;

    // INTERNAL USE UNIT TRAITS  -----------------------------------------------

    trait UnitInternal<T> where 
        Self: Clone + Copy + Sized,
        T: Clone + Copy + PartialEq + fmt::Display + fmt::Debug {

        fn new(value: f32, unit: T) -> Self;
        fn value(&self) -> f32;
        fn unit(&self) -> T;
        fn convert(&self, unit: T) -> Self;

        fn string_with_unit(&self) -> String {
            format!("{:.1} {}", self.value(), UnitInternal::unit(self))
        }
        fn value_in(&self, unit: T) -> f32 { // get the value of a unit in some other unit
            let a = self.clone();
            UnitInternal::convert(&a, unit).value()
        }
    }

    // PROPORTIONAL UNIT STRUCT ------------------------------------------------
    // helpful for building most units (where the conversion between units are proportional)

    #[derive(Clone, Copy, Debug)]
    pub struct ProportionalUnit<T> 
        where T: Proportional + Clone + Copy + fmt::Display + fmt::Debug {
        value: f32,
        unit: T,
    }
    pub trait Proportional {
        fn coefficient(&self) -> f32; // the coefficient that when multiplied by
                           // the value would convert this unit into the 
                           // "default" unit.
    }
    impl<T> UnitInternal<T> for ProportionalUnit<T> where 
        T: Proportional + Clone + Copy + PartialEq + Eq + fmt::Display + fmt::Debug {
        
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

    // UNITS -------------------------------------------------------------------

    // WIND ----------------------------------------------------------------
    pub type Speed = ProportionalUnit<SpeedUnit>;

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Display)]
    #[allow(unused)]
    pub enum SpeedUnit {
        #[strum(to_string = "mph")]
        Mph, 
        #[strum(to_string = "kph")]
        Kph, 
        #[strum(to_string = "kts")]
        Kts,
        #[strum(to_string = "m/s")]
        Mps,  
    }
    pub use SpeedUnit::*;

    impl Proportional for SpeedUnit {
        fn coefficient(&self) -> f32 {
            match self {
                Kph => 1.,
                Mph => 1.609344,
                Kts => 1.852,
                Mps => 3.6,
            }
        }
    }

    // PRESSURE ----------------------------------------------------------------
    pub type Pressure = ProportionalUnit<PressureUnit>;

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Display)]
    #[allow(unused)]
    pub enum PressureUnit {
        #[strum(to_string = "hPa")]
        HPa, 
        #[strum(to_string = "mbar")]
        Mbar, 
        #[strum(to_string = "inHg")]
        InHg,
        #[strum(to_string = "psi")]
        Psi,  
        #[strum(to_string = "atm")]
        Atm,  
    }
    pub use PressureUnit::*;

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

    // TEMPERATURE -------------------------------------------------------------
    // Not a proportional unit

    #[derive(Clone, Copy)]
    pub struct Temperature {
        value: f32,
        unit: TemperatureUnit
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug, Display)]
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


    // UNIT TRAIT IMPLEMENTATIONS ----------------------------------------------
    // (boring paperwork to connect the above traits together)

    impl<T, U: UnitInternal<T>> Unit<T> for U where     
        Self: Clone + Copy + Sized,
        T: Clone + Copy + PartialEq + Eq +fmt::Display + fmt::Debug {
        // fn value(&self) -> f32 {U::value(&self)}
        fn new(value: f32, unit: T) -> Self {U::new(value, unit)}
        fn unit(&self) -> T {U::unit(&self)}
        fn convert(&self, unit: T) -> Self {U::convert(&self, unit)}
        fn string_with_unit(&self) -> String {U::string_with_unit(&self)}
        fn value_in(&self, unit: T) -> f32 {U::value_in(&self, unit)}
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
        for (_i, u) in [Mph, Kph, Kts, Mps, Kts, Kph, Mph].iter().enumerate() {
            
            s = s.convert(*u);
            assert!((s.value_in(Mph) - 180.).abs() < 0.01);
            assert!((s.value_in(Kph) - 289.68192).abs() < 0.01);
            assert!((s.value_in(Kts) - 156.4157235421).abs() < 0.001);
            assert!((s.value_in(Mps) - 80.4672).abs() < 0.001);
        }

        assert_eq!(s.string_with_unit(), "180.0 mph");
    }
    
}