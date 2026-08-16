use std::sync::Arc;

use rand::RngCore;

use crate::{aabb::AABB, interval::Interval, material::Material, ray::Ray, texture::TextureCoords, vector::V3};

pub mod sphere;
pub mod quad;
pub mod triangle;
pub mod translate;
pub mod rotate;
pub mod medium;

pub struct HitRecord {
    pub point: V3,
    pub normal: V3,
    pub t: f64,
    pub front_face: bool,
    pub material: Arc<dyn Material>,
    pub texture_coords: TextureCoords
}

impl HitRecord {
    pub fn new(
        point: V3, 
        ray: &Ray, 
        outward_normal: V3, 
        t: f64, 
        material: &Arc<dyn Material>, 
        texture_coords: TextureCoords
    ) -> Self {
        let front_face = ray.direction.dot(&outward_normal) < 0.;
        let normal = if front_face { outward_normal } else { -outward_normal };
        let material = Arc::clone(material);
        Self { point, normal, t, front_face, material, texture_coords }
    }
}

pub trait Hittable: Send + Sync {
    fn hit(&self, ray: &Ray, t: Interval, rng: &mut dyn RngCore) -> Option<HitRecord>;
    fn bounding_box(&self) -> &AABB;

    /// Density (solid angle, in the direction/PDF sense) of sampling this
    /// object as seen from `origin` looking along `direction`. Only
    /// meaningful for objects used as explicit light sources; the default
    /// of 0 marks "not a sampleable light".
    fn pdf_value(&self, _origin: V3, _direction: V3) -> f64 { 0.0 }

    /// A random direction from `origin` toward this object, distributed
    /// according to `pdf_value`. The default is an arbitrary placeholder —
    /// callers should never invoke it unless `pdf_value` is overridden too.
    fn random(&self, _origin: V3, _rng: &mut dyn RngCore) -> V3 { V3::new(1., 0., 0.) }
}

#[derive(Default)]
pub struct HittableList {
    pub objects: Vec<Box<dyn Hittable>>,
    pub bbox: AABB,
}

impl HittableList {
    pub fn new() -> Self { Self::default() }

    pub fn add(&mut self, hittable: Box<dyn Hittable>) {
        self.bbox = self.bbox.join(hittable.bounding_box());
        self.objects.push(hittable);
    }

    pub fn add_list(&mut self, hittable_list: Self) {
        for hittable in hittable_list.objects {
            self.add(hittable);
        }
    }
}

impl Hittable for HittableList {
    fn hit(&self, ray: &Ray, t: Interval, rng: &mut dyn RngCore) -> Option<HitRecord> {
        let mut closest = t.max;
        let mut hit_record = None;

        for hittable in &self.objects {
            if let Some(hr) = hittable.hit(ray, Interval::new(t.min, closest), rng) {
                closest = hr.t;
                hit_record = Some(hr);
            }
        }

        hit_record
    }

    fn bounding_box(&self) -> &AABB {
        &self.bbox
    }
}