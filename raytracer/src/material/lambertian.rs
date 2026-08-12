use std::sync::Arc;

use rand::RngCore;

use crate::{colour::Colour, hittable::HitRecord, material::Material, ray::Ray, texture::{Texture, solid_colour::SolidColour}, vector::{near_zero, random_unit_vector}};

#[derive(Debug)]
pub struct Lambertian {
    pub tex: Arc<dyn Texture>,
}

impl Lambertian {
    pub fn new(tex: &Arc<dyn Texture>) -> Self {
        Self { tex: Arc::clone(tex) }
    }
    pub fn from_solid(albedo: Colour) -> Self {
        Self { tex: Arc::new(SolidColour::new(albedo)) }
    }

    pub fn from_rgb(r: f64, g: f64, b: f64) -> Self {
        let colour = Colour::new(r, g, b);
        Self::from_solid(colour)
    }
}

impl Material for Lambertian {
    fn scatter(&self, r_in: &Ray, hr: &HitRecord, rng: &mut dyn RngCore) -> Option<(Ray, Colour)> {
        let scatter_direction = match hr.normal + random_unit_vector( rng) {
            | v if !near_zero(v) => v,
            | _ => hr.normal,
        };
        Some((Ray::new_with_time(hr.point, scatter_direction, r_in.time), self.tex.value(hr.texture_coords, hr.point)))
    }
}