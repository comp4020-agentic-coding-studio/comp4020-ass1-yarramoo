use std::io::Write;

use crate::{interval::Interval, vector::V3};

pub type Colour = V3;

pub fn linear_to_gamma(x: f64) -> f64 {
    if x > 0. {
        x.sqrt()
    } else {
        0.
    }
}

pub fn gamma_to_linear(c: u8) -> u8 {
    // First scale to [0. .. 1.]
    let c = c as f32 / 255.;
    ((c * c) * 255.) as u8
}

pub fn write_colour<T: Write>(h: &mut T, colour: &Colour) -> std::io::Result<usize> {
    let r = linear_to_gamma(colour.x);
    let g = linear_to_gamma(colour.y);
    let b = linear_to_gamma(colour.z);

    let intensity = Interval { min: 0.0, max: 0.999 };
    let rbyte = intensity.clamp(r) * 256.;
    let gbyte = intensity.clamp(g) * 256.;
    let bbyte = intensity.clamp(b) * 256.;

    h.write(format!("{} {} {}\n", rbyte as usize, gbyte as usize, bbyte as usize).as_bytes())
}