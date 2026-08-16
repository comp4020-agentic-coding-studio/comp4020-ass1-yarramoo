use std::sync::Arc;

use rand::{Rng, RngCore};

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

    fn pdf_value(&self, origin: V3, direction: V3) -> f64 {
        let ray = Ray::new(origin, direction);
        let Some(hr) = self.hit(&ray, Interval::new(0.001, f64::INFINITY), &mut rand::rng()) else {
            return 0.0;
        };

        let area = self.u.cross(&self.v).norm();
        let distance_squared = hr.t * hr.t * direction.norm_squared();
        let cosine = (direction.dot(&hr.normal) / direction.norm()).abs();

        distance_squared / (cosine * area)
    }

    fn random(&self, origin: V3, rng: &mut dyn RngCore) -> V3 {
        let a: f64 = rng.random();
        let b: f64 = rng.random();
        let p = self.q + (a * self.u) + (b * self.v);
        p - origin
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::lambertian::Lambertian;

    #[test]
    fn pdf_value_matches_the_closed_form_area_distance_cosine_formula() {
        // Unit quad in the z=0 plane, spanning (0,0,0)..(1,1,0), normal +z.
        let material: Arc<dyn Material> = Arc::new(Lambertian::from_rgb(1., 1., 1.));
        let quad = Quad::new(V3::new(0., 0., 0.), V3::new(1., 0., 0.), V3::new(0., 1., 0.), &material);

        let origin = V3::new(0.5, 0.5, 5.);
        let direction = V3::new(0., 0., -1.);

        // area = 1, t = 5, cosine = 1 ⇒ pdf = t² * |dir|² / (cosine * area) = 25.
        let expected = 25.0;
        let actual = quad.pdf_value(origin, direction);
        assert!((actual - expected).abs() < 1e-9, "actual was {actual}");
    }

    #[test]
    fn pdf_value_is_zero_when_the_ray_misses_the_quad() {
        let material: Arc<dyn Material> = Arc::new(Lambertian::from_rgb(1., 1., 1.));
        let quad = Quad::new(V3::new(0., 0., 0.), V3::new(1., 0., 0.), V3::new(0., 1., 0.), &material);

        let origin = V3::new(5., 5., 5.);
        let direction = V3::new(0., 0., -1.);

        assert_eq!(quad.pdf_value(origin, direction), 0.0);
    }
}
