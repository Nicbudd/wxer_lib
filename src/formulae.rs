use std::f32::consts::PI;

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

pub fn c_to_f<T: Into<f32>>(f: T) -> f32 {
    let f = f.into();
    (f * 9./5.) + 32.
}

pub fn f_to_c<T: Into<f32>>(c: T) -> f32 {
    let c: f32 = c.into();
    (c - 32.0) * 5./9.
}

pub fn kts_to_mph(f: f32) -> f32 {
    f/0.868976
}

pub fn kts_to_kph(f: f32) -> f32 {
    f/0.539957
}


#[allow(non_snake_case)]
pub fn hpa_to_inhg<T: Into<f32>>(c: T) -> f32 {
    c.into()*0.02952998057228486
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
