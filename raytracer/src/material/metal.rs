use rand::RngCore;

use crate::{colour::Colour, hittable::HitRecord, material::Material, ray::Ray, vector::{random_unit_vector, reflect}};


#[derive(Debug)]
pub struct Metal {
    albedo: Colour,
    fuzz: f64
}

impl Metal {
    pub fn new(albedo: Colour, fuzz: f64) -> Self {
        let fuzz = if fuzz < 1. { fuzz } else { 1. };
        Self { albedo, fuzz }
    }

    pub fn from_rgb((r, g, b): (f64, f64, f64), fuzz: f64) -> Self {
        let fuzz = if fuzz < 1. { fuzz } else { 1. };
        Self { albedo: Colour::new(r, g, b), fuzz }
    }
}

impl Material for Metal {
    fn scatter(&self, r_in: &Ray, hr: &HitRecord, rng: &mut dyn RngCore) -> Option<(Ray, Colour)> {
        let reflected = reflect(r_in.direction, hr.normal);
        let reflected = reflected.normalize() + (self.fuzz * random_unit_vector(rng));
        let scattered = Ray::new(hr.point, reflected);
        Some((scattered, self.albedo))
    }
}