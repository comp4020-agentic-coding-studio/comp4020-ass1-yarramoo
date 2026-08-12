use std::sync::Arc;

use rand::RngCore;

use crate::{aabb::AABB, hittable::{HitRecord, Hittable}, interval::Interval, ray::Ray, vector::V3};


pub struct Translate {
    object: Arc<dyn Hittable>,
    offset: V3,
    bbox: AABB,
}

impl Translate {
    pub fn new(object: &Arc<dyn Hittable>, offset: V3) -> Self {
        let bbox = *object.bounding_box() + offset;
        let object = Arc::clone(object);
        Self { object, offset, bbox }
    }
}

impl Hittable for Translate {
    fn hit(&self, ray: &Ray, t: Interval, rng: &mut dyn RngCore) -> Option<HitRecord> {
        let offset_ray = Ray::new_with_time(ray.origin - self.offset, ray.direction, ray.time);
        if let Some(mut hr) = self.object.hit(&offset_ray, t, rng) {
            hr.point += self.offset;
            Some(hr)
        } else {
            None
        }
    }

    fn bounding_box(&self) -> &AABB {
        &self.bbox
    }
}