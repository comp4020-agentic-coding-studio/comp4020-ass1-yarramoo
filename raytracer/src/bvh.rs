
use rand::RngCore;

use crate::{aabb::AABB, hittable::{HitRecord, Hittable}, interval::Interval, ray::Ray};

pub enum BVHNode {
    Leaf {
        object: Box<dyn Hittable>,
    },
    Node {
        left: Box<Self>,
        right: Box<Self>,
        bbox: AABB,
    }
}

impl BVHNode {
    pub fn new(mut objects: Vec<Box<dyn Hittable>>) -> Self {
        if objects.len() == 1 {
            return Self::Leaf {
                object: objects.pop().unwrap()
            }
        }
        // Get the overall bounding box
        let bb = objects
            .iter()
            .fold(AABB::empty(), |bb: AABB, object: &Box<dyn Hittable>| {
                bb.join(object.bounding_box())
            });
        
        let compare = bb.longest_axis_cmp();
        objects.sort_by(|a, b| compare(a.bounding_box(), b.bounding_box()));
        let right_objects = objects.split_off(objects.len() / 2);
        let left = Box::new(Self::new(objects));
        let right = Box::new(Self::new(right_objects));
        let bbox = left.bounding_box().join(right.bounding_box());
        Self::Node { left, right, bbox }
    }
}

impl Hittable for BVHNode {
    fn bounding_box(&self) -> &AABB {
        match self {
            Self::Leaf { object } => object.bounding_box(),
            Self::Node { left:_, right:_, bbox } => bbox,
        }
    }

    fn hit(&self, ray: &Ray, t: Interval, rng: &mut dyn RngCore) -> Option<HitRecord> {
        if !self.bounding_box().hit(ray, t) {
            return None;
        }

        match self {
            BVHNode::Leaf { object } => object.hit(ray, t, rng),
            BVHNode::Node { left, right, bbox:_ } => {
                let hr_left = left.hit(ray, t, rng);
                let hr_right = right.hit(ray, t, rng);

                match (hr_left, hr_right) {
                    (Some(hrl), Some(hrr)) => {
                        if hrl.t < hrr.t {
                            Some(hrl)
                        } else {
                            Some(hrr)
                        }
                    },
                    (Some(hr), _) => Some(hr),
                    (_, Some(hr)) => Some(hr),
                    _ => None  
                }
            }
        }
    }
}