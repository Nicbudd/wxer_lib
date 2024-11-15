use std::f32::consts::PI;

const R: f32 = 8.314462618; // molar gas constant, J/mol/K
#[allow(non_upper_case_globals)]
const g: f32 = 9.80665; // m/s^2
#[allow(non_upper_case_globals)]
const Md: f32 = 28.96546e-3; // kg/mol
#[allow(non_upper_case_globals)]
const Rd: f32 = R / Md; // (J K-1 kg-1)

pub fn rh_to_dewpoint(temp: f32, rh: f32) -> f32 {
    let t_c = f_to_c(temp);
    
    let beta = 17.62; // constant
    let lambda = 243.12; // degrees C
    
    let ln_rh = (rh/100.).ln();
    let temp_term = (beta*t_c)/(lambda+t_c);
    let combined_term = ln_rh + temp_term;

    let dp_c = (lambda*combined_term)/(beta-combined_term);

    c_to_f(dp_c)
}

pub fn c_to_f(f: f32) -> f32 {
    (f * 9./5.) + 32.
}

pub fn f_to_c(c: f32) -> f32 {
    (c - 32.0) * 5./9.
}

pub fn c_to_k(c: f32) -> f32 {
    c + 273.15
}

pub fn k_to_c(k: f32) -> f32 {
    k - 273.15
}

pub fn k_to_f(k: f32) -> f32 {
    c_to_f(k_to_c(k))
}

pub fn f_to_k(f: f32) -> f32 {
    c_to_k(f_to_c(f))
}

pub fn kts_to_mph(f: f32) -> f32 {
    f/0.868976
}

pub fn kts_to_kph(f: f32) -> f32 {
    f/0.539957
}


#[allow(non_snake_case)]
pub fn hpa_to_inhg(h: f32) -> f32 {
    h*0.02952998057228486
}

#[allow(non_snake_case)]
pub fn inhg_to_hpa(i: f32) -> f32 {
    i/0.02952998057228486
}

pub fn distance_between_coords_km(lat1: f32, long1: f32, lat2: f32, long2: f32) -> f32 {
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

    d
} 

pub fn altimeter_to_station(altimeter: f32, height: f32) -> f32 {
    let height = height as f64;
    let altimeter = altimeter as f64;

    const GAMMA: f64 = 6.5e-3; // standard atmospheric lapse rate in C/m, 
    const T0: f64 = 288.0; // standard atmospheric temperature in K
    const P0: f64 = 1013.25; // standard atmospheric pressure, in mbar
    #[allow(non_upper_case_globals)]
    const n: f64 = GAMMA*(Rd as f64)/(g as f64);
    let first_term = (P0.powf(n))*GAMMA*height/T0;

    let result = (altimeter.powf(n) - first_term).powf(1.0/n) + 0.3;
    // dbg!(n, first_term, result);
    result as f32
}

// temperature input as °F, height as m, 
pub fn altimeter_to_slp(altimeter: f32, height: f32, temperature: f32) -> f32 {
    let h: f32 = f_to_k(temperature)*Rd/g; // (m)
    // dbg!(altimeter, height, temperature, f_to_k(temperature), h);
    let station_pres = altimeter_to_station(altimeter, height) as f32;
    station_pres*((height/h).exp())
}

pub fn vapor_pressure(temperature_kelvin: f32) -> f32 {
    // source: https://atoc.colorado.edu/~cassano/wx_calculator/formulas/vaporPressure.html
    let t_c = k_to_c(temperature_kelvin);
    return 6.11*10.0_f32.powf(7.5*t_c / (237.7 + t_c))
}


pub fn mixing_ratio_g_kg(temperature_kelvin: f32, station_pressure: f32) -> f32 {
    // source: https://www.weather.gov/media/epz/wxcalc/mixingRatio.pdf
    let vapor_pressure = vapor_pressure(temperature_kelvin);
    return 621.97 * (vapor_pressure / (station_pressure - vapor_pressure));
}

// an approximation
pub fn lcl_temperature(temperature_kelvin_below_lcl: f32, dewpoint_kelvin: f32) -> f32 {
    return 1.0/((1.0/(dewpoint_kelvin-56.0)) + ((temperature_kelvin_below_lcl/dewpoint_kelvin).ln()/800.0)) + 56.0;
}



// an approximation
pub fn theta_e(temperature_kelvin_below_lcl: f32, dewpoint_kelvin: f32, station_pressure: f32) -> f32 {
    const p_0: f32 = 1000.0;
    // source: https://en.wikipedia.org/wiki/Equivalent_potential_temperature
    let vap_pres = vapor_pressure(dewpoint_kelvin);
    let t_l: f32 = lcl_temperature(temperature_kelvin_below_lcl, dewpoint_kelvin); // temperature at LCL
    let r: f32 = mixing_ratio_g_kg(dewpoint_kelvin, station_pressure) / 1000.0; // mixing ratio in kg/kg
    let theta_l: f32 = temperature_kelvin_below_lcl * 
                        ((p_0/(station_pressure - vap_pres)).powf(0.2854)) *
                        ((temperature_kelvin_below_lcl/t_l).powf(0.28*r)); // dry potential temperature at LCL
    let theta_e = theta_l * (((3036.0/t_l) - 1.78) * r * (1.0 + (0.448*r))).exp();

    return theta_e
}