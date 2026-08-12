use std::{cmp::Ordering, ops::Add};

use crate::{interval::Interval, ray::Ray, vector::V3};


#[derive(Clone, Copy, Default, Debug)]
pub struct AABB {
    pub x: Interval,
    pub y: Interval,
    pub z: Interval,
}

impl AABB {
    pub fn new(x: Interval, y: Interval, z: Interval) -> Self {
        const DELTA: f64 = 0.0001;
        let pad = |interval: Interval| if interval.size() < DELTA { 
            interval.expand(DELTA) } else { interval };
        let x = pad(x);
        let y = pad(y);
        let z = pad(z);
        Self { x, y, z }
    }

    pub fn empty() -> Self {
        Self::new(Interval::EMPTY, Interval::EMPTY, Interval::EMPTY)
    }

    pub fn from_points(a: V3, b: V3) -> Self {
        let x = Interval::new_sort(a.x, b.x);
        let y = Interval::new_sort(a.y, b.y);
        let z = Interval::new_sort(a.z, b.z);
        Self { x, y, z }
    }

    pub fn join(&self, b: &Self) -> Self {
        Self::new(
            self.x.union(&b.x),
            self.y.union(&b.y),
            self.z.union(&b.z)
        )
    }

    pub fn hit(&self, ray: &Ray, t: Interval) -> bool {
        let intersection_1d = |ray_dir: f64, ray_orig: f64, interval: Interval| {
            let adinv = 1. / ray_dir;

            let t0 = (interval.min - ray_orig) * adinv;
            let t1 = (interval.max - ray_orig) * adinv;

            Interval::new_sort(t0, t1)
        };

        let x_intersect = intersection_1d(ray.direction.x, ray.origin.x, self.x);
        let y_intersect = intersection_1d(ray.direction.y, ray.origin.y, self.y);
        let z_intersect = intersection_1d(ray.direction.z, ray.origin.z, self.z);

        t.intersection(&x_intersect)
            .and_then(|t| y_intersect.intersection(&t))
            .and_then(|t| z_intersect.intersection(&t))
            .is_some()
    }

    pub fn cmp_x(a: &Self, b: &Self) -> Ordering {
        a.x.min.total_cmp(&b.x.min)
    }

    pub fn cmp_y(a: &Self, b: &Self) -> Ordering {
        a.y.min.total_cmp(&b.y.min)
    }

    pub fn cmp_z(a: &Self, b: &Self) -> Ordering {
        a.z.min.total_cmp(&b.z.min)
    }

    pub fn longest_axis_cmp(&self) -> fn(&AABB, &AABB) -> Ordering {
        if self.x.size() > self.y.size() {
            if self.x.size() > self.z.size() {
                Self::cmp_x
            } else {
                Self::cmp_z
            }
        } else if self.y.size() > self.z.size() {
            Self::cmp_y
        } else {
            Self::cmp_z
        }
        
    }
}

impl Add<AABB> for V3 {
    type Output = AABB;

    fn add(self, rhs: AABB) -> Self::Output {
        AABB::new(
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z
        )
    }
}

impl Add<V3> for AABB {
    type Output = Self;

    fn add(self, rhs: V3) -> Self::Output {
        AABB::new(
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z
        )
    }
}