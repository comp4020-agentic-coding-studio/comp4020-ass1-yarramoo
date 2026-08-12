use rand::RngCore;

use crate::{colour::Colour, hittable::HitRecord, ray::Ray, texture::TextureCoords, vector::V3};

pub trait Material: Send + Sync + std::fmt::Debug {
    fn scatter(&self, _r_in: &Ray, _hr: &HitRecord, _rng: &mut dyn RngCore) -> Option<(Ray, Colour)> { None }
    fn emitted(&self, _texture_coords: TextureCoords, _p: V3) -> Colour { Colour::new(0.,0.,0.) }
}

pub mod lambertian;
pub mod metal;
pub mod dielectric;
pub mod diffuse_light;
pub mod isotropic;