use std::{f64::consts::PI, sync::Arc};

use rand::RngCore;

use crate::{aabb::AABB, hittable::{HitRecord, Hittable}, interval::Interval, material::Material, ray::Ray, vector::{V2, V3}};


pub struct Sphere {
    centre: Ray,
    radius: f64,
    material: Arc<dyn Material>,
    bbox: AABB
}

impl Sphere {
    pub fn new(centre: V3, radius: f64, material: &Arc<dyn Material>) -> Self {
        let bbox = AABB::from_points(centre.add_scalar(-radius), centre.add_scalar(radius));
        let centre = Ray::new(centre, V3::default());
        Self { centre, radius: radius.max(0.), material: Arc::clone(material), bbox }
    }

    pub fn new_moving(centre1: V3, centre2: V3, radius: f64, material: &Arc<dyn Material>) -> Self {
        let bbox1 = AABB::from_points(centre1.add_scalar(-radius), centre1.add_scalar(radius));
        let bbox2 = AABB::from_points(centre2.add_scalar(-radius), centre2.add_scalar(radius));
        let bbox = bbox1.join(&bbox2);
        Self {
            centre: Ray::new(centre1, centre2-centre1),
            radius: radius.max(0.),
            material: Arc::clone(material),
            bbox,
        }
    }

    pub fn get_uv(point: V3) -> V2 {
        let theta = (-point.y).acos();
        let phi = f64::atan2(-point.z, point.x) + PI;
        
        let u = phi / (2.*PI);
        let v = theta / PI; 
        V2::new(u, v)
    }
}

impl Hittable for Sphere {
    fn hit(&self, ray: &Ray, t: Interval, _rng: &mut dyn RngCore) -> Option<HitRecord> {
        let current_centre = self.centre.at(ray.time);
        let oc = current_centre - ray.origin;
        let a = ray.direction.norm_squared();
        let h = ray.direction.dot(&oc);
        let c = oc.norm_squared() - self.radius*self.radius;

        let discriminant = h*h - a*c;
        if discriminant < 0. { return None; }

        let sqrtd = discriminant.sqrt();

        let root = {
            let mut root = (h - sqrtd) / a;
            if root <= t.min || root >= t.max {
                root = (h + sqrtd) / a;
            }
            if root <= t.min || root >= t.max {
                None
            } else {
                Some(root)
            }
        }?;

        let point = ray.at(root);
        let outward_normal = (point - current_centre) / self.radius;
        let texture_coords = Self::get_uv(outward_normal);
        Some(HitRecord::new(
            point,
            ray,
            outward_normal,
            root,
            &self.material,
            texture_coords,
        ))
    }
    
    fn bounding_box(&self) -> &AABB {
        &self.bbox
    }
}