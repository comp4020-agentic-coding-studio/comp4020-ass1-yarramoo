use rand::{Rng, RngCore};

use crate::{colour::Colour, hittable::HitRecord, material::Material, ray::Ray, vector::{reflect, refract}};

#[derive(Debug)]
pub struct Dialectric {
    refraction_index: f64,
}

impl Dialectric {
    pub fn new(refraction_index: f64) -> Self {
        Self { refraction_index }
    }
}

impl Material for Dialectric {
    fn scatter(&self, r_in: &Ray, hr: &HitRecord, rng: &mut dyn RngCore) -> Option<(Ray, Colour)> {
        let ri = if hr.front_face {
            1. / self.refraction_index
        } else {
            self.refraction_index
        };

        let unit_direction = r_in.direction.normalize();
        let cos_theta = hr.normal.dot(&-unit_direction).min(1.);
        let sin_theta = (1. - cos_theta*cos_theta).sqrt();

        let cannot_refract = ri * sin_theta > 1.;
        let direction = if cannot_refract || reflectance(cos_theta, ri) > rng.random() {
            reflect(unit_direction, hr.normal)
        } else {
            refract(unit_direction, hr.normal, ri)
        };

        Some((Ray::new_with_time(hr.point, direction, r_in.time), Colour::new(1., 1., 1.)))
    }
}

fn reflectance(cosine: f64, refraction_index: f64) -> f64 {
    // The whole fraction is squared (Schlick's approximation), not just the
    // denominator — a bare `.powi(2)` here binds tighter than `/` and would
    // silently compute (1-n)/(1+n)^2 instead of ((1-n)/(1+n))^2.
    let r0 = ((1. - refraction_index) / (1. + refraction_index)).powi(2);
    r0 + (1.-r0) * (1. - cosine).powi(5)
}