use std::sync::Arc;

use rand::RngCore;

use crate::{aabb::AABB, hittable::{HitRecord, Hittable}, interval::Interval, material::Material, ray::Ray, texture::TextureCoords, vector::V3};

pub struct Triangle {
    a: V3,
    b: V3,
    c: V3,
    normal: V3,
    material: Arc<dyn Material>,
    bbox: AABB,
}

impl Triangle {
    pub fn new(a: V3, b: V3, c: V3, material: &Arc<dyn Material>) -> Self {
        let u = b - a;
        let v = b - c;
        let normal = u.cross(&v).normalize();
        let material = Arc::clone(material);
        let bbox = Self::make_bbox(a, b, c);
        Self { a, b, c, normal, material, bbox }
    }

    fn make_bbox(a: V3, b: V3, c: V3) -> AABB {
        let bbox1 = AABB::from_points(a, b);
        let bbox2 = AABB::from_points(a, c);
        bbox1.join(&bbox2)
    }
}

impl Hittable for Triangle {
    fn hit(&self, ray: &Ray, t: Interval, _rng: &mut dyn RngCore) -> Option<HitRecord> {
        let ab = self.b - self.a;
        let ac = self.c - self.a;
        let pvec = ray.direction.cross(&ac);
        let det = ab.dot(&pvec);

        if det.abs() < 1e-8 {
            return None;
        }

        let inv_det = 1. / det;

        let tvec = ray.origin - self.a;
        let u = tvec.dot(&pvec) * inv_det;
        if u < 0. || u > 1. {
            return None;
        }

        let qvec = tvec.cross(&ab);
        let v = ray.direction.dot(&qvec) * inv_det;
        if v < 0. || u + v > 1. {
            return None;
        }

        let intersection_t = ac.dot(&qvec) * inv_det;
        if !t.surrounds(intersection_t) {
            return None;
        }

        Some(HitRecord::new(
            ray.at(intersection_t),
            ray,
            self.normal,
            intersection_t,
            &self.material,
            TextureCoords::new(u, v)
        ))
    }

    fn bounding_box(&self) -> &AABB {
        &self.bbox
    }
}


