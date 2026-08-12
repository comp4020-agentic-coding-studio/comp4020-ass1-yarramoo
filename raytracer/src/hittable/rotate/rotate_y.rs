use std::sync::Arc;

use rand::RngCore;

use crate::{aabb::AABB, hittable::{HitRecord, Hittable}, interval::Interval, ray::Ray, vector::V3};

pub struct RotateY {
    object: Arc<dyn Hittable>,
    sin_theta: f64,
    cos_theta: f64,
    bbox: AABB,
}

impl RotateY {
    pub fn new(object: &Arc<dyn Hittable>, angle: f64) -> Self {
        let object = Arc::clone(object);
        let radians = angle.to_radians();
        let sin_theta = radians.sin();
        let cos_theta = radians.cos();
        let bbox = object.bounding_box();

        let mut min = V3::new(f64::INFINITY, f64::INFINITY, f64::INFINITY);
        let mut max = V3::new(f64::NEG_INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

        for i in 0..2 {
            let i = i as f64;
            let x = i*bbox.x.max + (1.-i)*bbox.x.min;

            for j in 0..2 {
                let j = j as f64;
                let y = j*bbox.y.max + (1.-j)*bbox.y.min;

                for k in 0..2 {
                    let k = k as f64;
                    let z = k*bbox.z.max + (1.-k)*bbox.z.min;

                    let newz = -sin_theta*x + cos_theta*z;
                    let newx = cos_theta*x + sin_theta*z;

                    let tester = V3::new(newx, y, newz);

                    min = V3::new(
                        f64::min(min.x, tester.x), 
                        f64::min(min.y, tester.y),
                        f64::min(min.z, tester.z)
                    );
                    max = V3::new(
                        f64::max(max.x, tester.x), 
                        f64::max(max.y, tester.y),
                        f64::max(max.z, tester.z)
                    );
                }
            }
        }

        let bbox = AABB::from_points(min, max);
        Self { object, sin_theta, cos_theta, bbox }
    }
}

impl Hittable for RotateY {
    fn hit(&self, ray: &Ray, t: Interval, rng: &mut dyn RngCore) -> Option<HitRecord> {
        let cos_theta = self.cos_theta;
        let sin_theta = self.sin_theta;

        let to_object_space = |v: V3| {
            V3::new(
                (cos_theta * v.x) - (sin_theta * v.z),
                v.y,
                (sin_theta * v.x) + (cos_theta * v.z)
            )
        };
        let origin = to_object_space(ray.origin);
        let direction = to_object_space(ray.direction);
        let rotated_ray = Ray::new_with_time(origin, direction, ray.time);

        let opt_hr = self.object.hit(&rotated_ray, t, rng);
        if opt_hr.is_none() {
            return None;
        }
        let mut hr = opt_hr.unwrap();

        let to_world_space = |v: V3| {
            V3::new(
                (cos_theta * v.x) + (sin_theta * v.z),
                v.y,
                -(sin_theta * v.x) + (cos_theta * v.z)
            )
        };

        hr.point = to_world_space(hr.point);
        hr.normal = to_world_space(hr.normal);

        Some(hr)
    }

    fn bounding_box(&self) -> &AABB {
        &self.bbox
    }
}


// class rotate_y : public hittable {
//     public:
  
//       bool hit(const ray& r, interval ray_t, hit_record& rec) const override {
  
  
//           rec.p = point3(
//               (cos_theta * rec.p.x()) + (sin_theta * rec.p.z()),
//               rec.p.y(),
//               (-sin_theta * rec.p.x()) + (cos_theta * rec.p.z())
//           );
  
//           rec.normal = vec3(
//               (cos_theta * rec.normal.x()) + (sin_theta * rec.normal.z()),
//               rec.normal.y(),
//               (-sin_theta * rec.normal.x()) + (cos_theta * rec.normal.z())
//           );
  
//           return true;
//       }
//   };