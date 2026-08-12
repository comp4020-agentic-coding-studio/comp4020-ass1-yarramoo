use std::sync::Arc;

use rand::RngCore;

use crate::{aabb::AABB, hittable::{HitRecord, Hittable, HittableList}, interval::Interval, material::Material, ray::Ray, texture::TextureCoords, vector::V3, world_add};

#[derive(Debug)]
pub struct Quad {
    q: V3, // Point 1
    u: V3, // Point 2
    v: V3, // Point 3
    w: V3,
    material: Arc<dyn Material>,
    bbox: AABB, 
    normal: V3,
    d: f64,
}

impl Quad {
    pub fn new(q: V3, u: V3, v: V3, material: &Arc<dyn Material>) -> Self {
        let n = u.cross(&v);
        let normal = n.normalize();
        let d = normal.dot(&q);
        Self {
            q, 
            u, 
            v,
            w: n / n.dot(&n),
            material: Arc::clone(material),
            bbox: Self::bounding_box(q, u, v),
            normal,
            d
        }
    }

    fn bounding_box(q: V3, u: V3, v: V3) -> AABB {
        let bbox1 = AABB::from_points(q, q + u + v);
        let bbox2 = AABB::from_points(q + v, q + u);
        bbox1.join(&bbox2)
    }
}

impl Hittable for Quad {
    fn hit(&self, ray: &Ray, t: Interval, _rng: &mut dyn RngCore) -> Option<HitRecord> {
        let denom = self.normal.dot(&ray.direction);

        if denom.abs() < 1e-8 {
            return None;
        }

        let quad_t = (self.d - self.normal.dot(&ray.origin)) / denom;
        if !t.contains(quad_t) {
            return None;
        }

        let intersection = ray.at(quad_t);
        let planar_hitpoint = intersection - self.q;

        let alpha = self.w.dot(&planar_hitpoint.cross(&self.v));
        let beta = self.w.dot(&self.u.cross(&planar_hitpoint));

        if !Interval::UNIT.contains(alpha) || !Interval::UNIT.contains(beta) {
            return None;
        }

        Some(HitRecord::new(
            intersection,
            ray,
            self.normal,
            quad_t,
            &self.material,
            TextureCoords::new(alpha, beta),
        ))
    }

    fn bounding_box(&self) -> &AABB {
        &self.bbox
    }
}

pub fn quad_box(a: V3, b: V3, material: &Arc<dyn Material>) -> HittableList {
    let mut sides = HittableList::new();

    let min = V3::new(f64::min(a.x, b.x), f64::min(a.y, b.y), f64::min(a.z, b.z));
    let max = V3::new(f64::max(a.x, b.x), f64::max(a.y, b.y), f64::max(a.z, b.z));

    let dx = V3::new(max.x - min.x, 0., 0.);
    let dy = V3::new(0., max.y - min.y, 0.);
    let dz = V3::new(0., 0., max.z - min.z);

    world_add!(Quad sides, (min.x, min.y, max.z), (dx.x, dx.y, dx.z), (dy.x, dy.y, dy.z), material);
    world_add!(Quad sides, (max.x, min.y, max.z), (-dz.x, -dz.y, -dz.z), (dy.x, dy.y, dy.z), material);
    world_add!(Quad sides, (max.x, min.y, min.z), (-dx.x, -dx.y, -dx.z), (dy.x, dy.y, dy.z), material);
    world_add!(Quad sides, (min.x, min.y, min.z), ( dz.x,  dz.y,  dz.z), (dy.x, dy.y, dy.z), material);
    world_add!(Quad sides, (min.x, max.y, max.z), ( dx.x,  dx.y,  dx.z), (-dz.x, -dz.y, -dz.z), material);
    world_add!(Quad sides, (min.x, min.y, min.z), ( dx.x,  dx.y,  dx.z), (dz.x, dz.y, dz.z), material);

    sides
}
