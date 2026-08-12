use std::sync::Arc;

use rand::RngCore;

use crate::{colour::Colour, hittable::HitRecord, material::Material, ray::Ray, texture::{Texture, TextureCoords, solid_colour::SolidColour}, vector::{V3, random_unit_vector}};

#[derive(Debug)]
pub struct Isotropic {
    tex: Arc<dyn Texture>,
}

impl Isotropic {
    pub fn new(texture: &Arc<dyn Texture>) -> Self {
        Self { tex: Arc::clone(texture) }
    }
    
    pub fn from_albedo(albedo: Colour) -> Self {
        Self { tex: Arc::new(SolidColour::new(albedo)) }
    }
}

impl Material for Isotropic {
    fn scatter(&self, r_in: &Ray, hr: &HitRecord, rng: &mut dyn RngCore) -> Option<(Ray, Colour)> {
        let scattered = Ray::new_with_time(hr.point, random_unit_vector(rng), r_in.time);
        let attenuation = self.tex.value(hr.texture_coords, hr.point);
        Some((scattered, attenuation))
    }
    
    fn emitted(&self, _texture_coords: TextureCoords, _p: V3) -> Colour { Colour::new(0.,0.,0.) }
}