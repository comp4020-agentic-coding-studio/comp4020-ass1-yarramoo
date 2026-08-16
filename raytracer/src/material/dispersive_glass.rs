use rand::{Rng, RngCore};

use crate::{colour::Colour, hittable::HitRecord, material::Material, ray::Ray, vector::{reflect, refract}};

/// Glass with a different refraction index per RGB channel — real dispersion,
/// approximated in three channels instead of a full spectrum. `scatter`
/// renders correctly in the full progressive image: each call picks one of
/// the three channels uniformly at random and returns that channel's pure
/// colour scaled by 3 (an unbiased Monte-Carlo estimator — over many
/// samples the three channels average back out to full white), so glass
/// edges pick up real chromatic fringing as the render converges.
///
/// Unlike `Dialectric`, this never partially reflects at grazing angles
/// (no Schlick term) — only total-internal-reflection forces a reflection.
/// That keeps `scatter_dispersive` (used by the trace-dot visualization)
/// fully deterministic per channel, so clicking the prism reliably shows
/// all three colours splitting apart rather than leaving it to chance.
#[derive(Debug)]
pub struct DispersiveGlass {
    ior: [f64; 3],
}

impl DispersiveGlass {
    pub fn new(ior_r: f64, ior_g: f64, ior_b: f64) -> Self {
        Self { ior: [ior_r, ior_g, ior_b] }
    }

    /// A centred refraction index plus a spread — red bends least, blue most.
    pub fn from_spread(centre_ior: f64, spread: f64) -> Self {
        Self::new(centre_ior - spread, centre_ior, centre_ior + spread)
    }

    fn refract_channel(&self, r_in: &Ray, hr: &HitRecord, channel: usize) -> (Ray, Colour) {
        let ior = self.ior[channel];
        let ri = if hr.front_face { 1. / ior } else { ior };

        let unit_direction = r_in.direction.normalize();
        let cos_theta = hr.normal.dot(&-unit_direction).min(1.);
        let sin_theta = (1. - cos_theta * cos_theta).sqrt();
        let cannot_refract = ri * sin_theta > 1.;

        let direction = if cannot_refract {
            reflect(unit_direction, hr.normal)
        } else {
            refract(unit_direction, hr.normal, ri)
        };

        let mut attenuation = Colour::new(0., 0., 0.);
        attenuation[channel] = 3.;
        (Ray::new_with_time(hr.point, direction, r_in.time), attenuation)
    }
}

impl Material for DispersiveGlass {
    fn scatter(&self, r_in: &Ray, hr: &HitRecord, rng: &mut dyn RngCore) -> Option<(Ray, Colour)> {
        let channel = rng.random_range(0..3usize);
        Some(self.refract_channel(r_in, hr, channel))
    }

    fn scatter_dispersive(&self, r_in: &Ray, hr: &HitRecord, channel: usize) -> Option<(Ray, Colour)> {
        Some(self.refract_channel(r_in, hr, channel.min(2)))
    }
}
