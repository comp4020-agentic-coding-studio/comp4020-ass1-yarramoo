use rand::RngCore;

use crate::{colour::Colour, hittable::HitRecord, ray::Ray, texture::TextureCoords, vector::V3};

pub trait Material: Send + Sync + std::fmt::Debug {
    fn scatter(&self, _r_in: &Ray, _hr: &HitRecord, _rng: &mut dyn RngCore) -> Option<(Ray, Colour)> { None }
    fn emitted(&self, _texture_coords: TextureCoords, _p: V3) -> Colour { Colour::new(0.,0.,0.) }

    /// Deterministic single-wavelength-channel scatter (0=red, 1=green,
    /// 2=blue), for materials whose real `scatter` splits light stochastically
    /// by channel (see `dispersive_glass`). Every other material keeps the
    /// default `None` — "I don't split by channel" — so this is free to
    /// implement and has no effect on the beauty render; it exists purely so
    /// a caller (the wasm trace-dot visualization) can force all three
    /// channels to appear deterministically instead of leaving it to chance.
    fn scatter_dispersive(&self, _r_in: &Ray, _hr: &HitRecord, _channel: usize) -> Option<(Ray, Colour)> { None }

    /// Whether this material's `scatter` already returns a fully-formed
    /// ray+attenuation with no importance-sampling PDF behind it (mirrors
    /// the reflective/refractive materials in the book). Defaults to `true`
    /// so every existing material — Metal, Dielectric, Isotropic,
    /// DispersiveGlass — keeps behaving exactly as it does today; only
    /// `Lambertian` overrides this to opt into PDF-based sampling.
    fn is_specular(&self) -> bool { true }

    /// Probability density (over solid angle) of scattering into
    /// `scattered`, used only for non-specular materials. Meaningless (and
    /// unused) while `is_specular()` is true.
    fn scattering_pdf(&self, _r_in: &Ray, _hr: &HitRecord, _scattered: &Ray) -> f64 { 0.0 }
}

pub mod lambertian;
pub mod metal;
pub mod dielectric;
pub mod diffuse_light;
pub mod isotropic;
pub mod dispersive_glass;