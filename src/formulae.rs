use std::f32::consts::PI;

use crate::units::*;

const R: f32 = 8.314462618; // molar gas constant, J/mol/K
#[allow(non_upper_case_globals)]
const g: f32 = 9.80665; // m/s^2
#[allow(non_upper_case_globals)]
const Md: f32 = 28.96546e-3; // kg/mol
#[allow(non_upper_case_globals)]
const Rd: f32 = R / Md; // (J K-1 kg-1)

pub fn rh_to_dewpoint(temp: Temperature, rh: Fractional) -> Temperature {
    let t_c = temp.value_in(Celsius);
    
    let beta = 17.62; // constant
    let lambda = 243.12; // degrees C
    
    let ln_rh = rh.value_in(Decimal).ln();
    let temp_term = (beta*t_c)/(lambda+t_c);
    let combined_term = ln_rh + temp_term;

    let dp_c = (lambda*combined_term)/(beta-combined_term);

    Temperature::new(dp_c, Celsius)
}

pub fn distance_between_coords(lat1: f32, long1: f32, lat2: f32, long2: f32) -> Distance {
    // Haversine formula
    // assuming symmetrical earth
    let earth_radius = 6371.0; // km, approx
    let phi_1 = lat1 * PI / 180.;
    let phi_2 = lat2 * PI / 180.;
    let delta_phi = (lat2-lat1) * PI / 180.;
    let delta_lmbda = (long2-long1) * PI / 180.;

    let a = (delta_phi/2.).sin() * (delta_phi/2.).sin() + 
    phi_1.cos() * phi_2.cos() * 
    (delta_lmbda / 2.).sin() * (delta_lmbda / 2.).sin();

    let c = 2. * (a.sqrt()).atan2((1.-a).sqrt());

    let d = earth_radius  * c;

    Distance::new(d, Kilometer)
} 

pub fn altimeter_to_station(altimeter: Pressure, height: Altitude) -> Pressure {
    let height = height.value_in(Meter);
    let altimeter = altimeter.value_in(Mbar);

    const GAMMA: f32 = 6.5e-3; // standard atmospheric lapse rate in C/m, 
    const T0: f32 = 288.0; // standard atmospheric temperature in K
    const P0: f32 = 1013.25; // standard atmospheric pressure, in mbar
    #[allow(non_upper_case_globals)]
    const n: f32 = GAMMA*Rd/g;
    let first_term = (P0.powf(n))*GAMMA*height/T0;

    let result = (altimeter.powf(n) - first_term).powf(1.0/n) + 0.3;
    // dbg!(n, first_term, result);
    Pressure::new(result, HPa)
}

// temperature input as °F, height as m, 
pub fn altimeter_to_slp(altimeter: Pressure, height: Altitude, temperature: Temperature) -> Pressure {
    let h: f32 = temperature.value_in(Kelvin)*Rd/g; // (m)
    let station_pres = altimeter_to_station(altimeter, height).value_in(HPa);
    let slp = station_pres*(( height.value_in(Meter) / h ).exp());
    Pressure::new(slp, HPa)
}


pub fn vapor_pressure(temperature: Temperature) -> Pressure {
    // source: https://atoc.colorado.edu/~cassano/wx_calculator/formulas/vaporPressure.html
    let t_c = temperature.value_in(Celsius);
    let value = 6.11*10.0_f32.powf(7.5*t_c / (237.7 + t_c));
    Pressure::new(value, HPa)
}

pub fn mixing_ratio(temperature: Temperature, station_pressure: Pressure) -> Fractional {
    // returns in units of kg/kg
    // source: https://www.weather.gov/media/epz/wxcalc/mixingRatio.pdf
    let e: f32 = vapor_pressure(temperature).value_in(HPa);
    let p_sta = station_pressure.value_in(HPa);
    let g_kg =  621.97 * (e / (p_sta - e));
    return Fractional::new(g_kg/1000., Decimal)
}

// an approximation
pub fn lcl_temperature(temperature_below_lcl: Temperature, dewpoint: Temperature) -> Temperature {
    let t = temperature_below_lcl.value_in(Kelvin);
    let td = dewpoint.value_in(Kelvin);
    let value =  1.0/((1.0/(td-56.0)) + ((t/td).ln()/800.0)) + 56.0;
    return Temperature::new(value, Kelvin)
}

// an approximation
pub fn theta_e(temperature_below_lcl: Temperature, dewpoint: Temperature, station_pressure: Pressure) -> Temperature {
    // source: https://en.wikipedia.org/wiki/Equivalent_potential_temperature
    const P0: f32 = 1000.0; // reference pressure (hPa)
    
    let t = temperature_below_lcl.value_in(Kelvin);
    let p = station_pressure.value_in(HPa);
    let e = vapor_pressure(dewpoint).value_in(HPa);
    let t_l = lcl_temperature(temperature_below_lcl, dewpoint).value_in(Kelvin); // temperature at LCL
    let r = mixing_ratio(dewpoint, station_pressure).value_in(Decimal); // mixing ratio in kg/kg

    let theta_l: f32 = t * ((P0/(p - e)).powf(0.2854)) * ((t/t_l).powf(0.28*r)); // dry potential temperature at LCL

    let theta_e = theta_l * (((3036.0/t_l) - 1.78) * r * (1.0 + (0.448*r))).exp();

    return Temperature::new(theta_e, Kelvin)
}