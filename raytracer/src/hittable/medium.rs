use std::{f64::INFINITY, sync::Arc};

use rand::{Rng, RngCore};

use crate::{aabb::AABB, colour::Colour, hittable::{HitRecord, Hittable}, interval::Interval, material::{Material, isotropic::Isotropic}, ray::Ray, texture::Texture, vector::V3};


pub struct ConstantMedium {
    boundary: Arc<dyn Hittable>,
    neg_inv_density: f64,
    phase_function: Arc<dyn Material>,
}

impl ConstantMedium {
    pub fn new(boundary: &Arc<dyn Hittable>, density: f64, tex: &Arc<dyn Texture>) -> Self {
        Self {
            boundary: Arc::clone(boundary),
            neg_inv_density: -1. / density,
            phase_function: Arc::new(Isotropic::new(tex)),
        }
    }

    pub fn from_albedo(boundary: &Arc<dyn Hittable>, density: f64, albedo: Colour) -> Self {
        Self {
            boundary: Arc::clone(boundary),
            neg_inv_density: -1. / density,
            phase_function: Arc::new(Isotropic::from_albedo(albedo)),
        }
    }
}

impl Hittable for ConstantMedium {
    fn hit(&self, ray: &Ray, t: Interval, rng: &mut dyn RngCore) -> Option<HitRecord> {
        let hr1 = self.boundary.hit(ray, Interval::UNIVERSE, rng);
        if hr1.is_none() { 
            return None; 
        }
        let mut hr1 = hr1.unwrap();

        let hr2 = self.boundary.hit(ray, Interval::new(hr1.t+0.0001, INFINITY), rng);
        if hr2.is_none() { 
            return None; 
        }
        let mut hr2 = hr2.unwrap();

        if hr1.t < t.min { hr1.t = t.min; }
        if hr2.t > t.max { hr2.t = t.max; }

        if hr1.t >= hr2.t { 
            return None; 
        }

        if hr1.t < 0. {
            hr1.t = 0.;
        }

        let ray_length = ray.direction.norm();
        let distance_inside_boundary = (hr2.t - hr1.t) * ray_length;
        let hit_distance = self.neg_inv_density * rng.random::<f64>().ln();

        if hit_distance > distance_inside_boundary {
            return None;
        }

        Some(HitRecord::new(
            ray.at(hit_distance),
            ray,
            V3::new(1.,0.,0.),
            hr1.t + hit_distance / ray_length,
            &self.phase_function,
            hr1.texture_coords
        ))
    }

    fn bounding_box(&self) -> &AABB {
        self.boundary.bounding_box()
    }
}