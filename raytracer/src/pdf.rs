use std::f64::consts::PI;
use std::sync::Arc;

use rand::{Rng, RngCore};

use crate::hittable::Hittable;
use crate::onb::Onb;
use crate::vector::{V3, random_unit_vector};

pub trait Pdf: Send + Sync {
    fn value(&self, direction: V3) -> f64;
    fn generate(&self, rng: &mut dyn RngCore) -> V3;
}

fn random_cosine_direction(rng: &mut dyn RngCore) -> V3 {
    let r1: f64 = rng.random();
    let r2: f64 = rng.random();
    let phi = 2. * PI * r1;
    let z = (1. - r2).sqrt();
    let r2_sqrt = r2.sqrt();
    V3::new(phi.cos() * r2_sqrt, phi.sin() * r2_sqrt, z)
}

pub struct CosinePdf {
    uvw: Onb,
}

impl CosinePdf {
    pub fn new(normal: V3) -> Self {
        Self { uvw: Onb::from_normal(normal) }
    }
}

impl Pdf for CosinePdf {
    fn value(&self, direction: V3) -> f64 {
        let cosine_theta = direction.normalize().dot(&self.uvw.w());
        (cosine_theta / PI).max(0.)
    }

    fn generate(&self, rng: &mut dyn RngCore) -> V3 {
        self.uvw.local(random_cosine_direction(rng))
    }
}

pub struct UniformHemispherePdf {
    uvw: Onb,
}

impl UniformHemispherePdf {
    pub fn new(normal: V3) -> Self {
        Self { uvw: Onb::from_normal(normal) }
    }
}

impl Pdf for UniformHemispherePdf {
    fn value(&self, _direction: V3) -> f64 {
        1. / (2. * PI)
    }

    fn generate(&self, rng: &mut dyn RngCore) -> V3 {
        // random_unit_vector is isotropic over the whole sphere; flipping
        // into the basis's local +z hemisphere before transforming out to
        // world space gives a uniform direction over that hemisphere.
        let mut local = random_unit_vector(rng);
        if local.z < 0. {
            local.z = -local.z;
        }
        self.uvw.local(local)
    }
}

/// Wraps a set of sampleable "lights" as a single PDF: `value` averages the
/// per-object solid-angle density uniformly across the set, `generate` picks
/// one uniformly at random and samples toward it. With exactly one light
/// (every current scene has 0 or 1) this reduces to plain single-light NEE.
pub struct HittablePdf {
    objects: Vec<Arc<dyn Hittable>>,
    origin: V3,
}

impl HittablePdf {
    pub fn new(objects: Vec<Arc<dyn Hittable>>, origin: V3) -> Self {
        Self { objects, origin }
    }
}

impl Pdf for HittablePdf {
    fn value(&self, direction: V3) -> f64 {
        if self.objects.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.objects.iter().map(|o| o.pdf_value(self.origin, direction)).sum();
        sum / self.objects.len() as f64
    }

    fn generate(&self, rng: &mut dyn RngCore) -> V3 {
        let idx = rng.random_range(0..self.objects.len());
        self.objects[idx].random(self.origin, rng)
    }
}

pub struct MixturePdf {
    p: [Arc<dyn Pdf>; 2],
}

impl MixturePdf {
    pub fn new(p0: Arc<dyn Pdf>, p1: Arc<dyn Pdf>) -> Self {
        Self { p: [p0, p1] }
    }
}

impl Pdf for MixturePdf {
    fn value(&self, direction: V3) -> f64 {
        0.5 * self.p[0].value(direction) + 0.5 * self.p[1].value(direction)
    }

    fn generate(&self, rng: &mut dyn RngCore) -> V3 {
        if rng.random::<f64>() < 0.5 {
            self.p[0].generate(rng)
        } else {
            self.p[1].generate(rng)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn cosine_pdf_integrates_to_one_over_the_hemisphere() {
        // Monte-Carlo-check ∫_hemisphere pdf(w) dw ≈ 1 by importance-sampling
        // with a uniform-on-the-sphere distribution (density 1/4π) — since
        // CosinePdf::value is 0 for the opposite hemisphere, integrating the
        // ratio over the whole sphere gives the same answer as integrating
        // over just the hemisphere it's defined on.
        let mut rng = StdRng::seed_from_u64(42);
        let pdf = CosinePdf::new(V3::new(0., 0., 1.));
        let n = 200_000;
        let sum: f64 = (0..n).map(|_| pdf.value(random_unit_vector(&mut rng))).sum();
        let estimate = 4. * PI * sum / n as f64;
        assert!((estimate - 1.0).abs() < 0.05, "estimate was {estimate}");
    }
}
