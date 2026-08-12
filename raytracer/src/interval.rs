use std::ops::Add;

use rand::{Rng, RngCore};


#[derive(Clone, Copy, Debug)]
pub struct Interval {
    pub min: f64,
    pub max: f64,
}

impl Interval {
    pub const EMPTY: Self = Self {
        min: f64::MAX,
        max: f64::MIN
    };

    pub const UNIVERSE: Self = Self {
        min: f64::MIN,
        max: f64::MAX
    };

    pub const UNIT: Self = Self {
        min: 0.,
        max: 1.,
    };

    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }
    
    pub fn new_sort(a: f64, b: f64) -> Self {
        if a < b {
            Self::new(a, b)
        } else {
            Self::new(b, a)
        }
    }

    pub fn size(&self) -> f64 {
        self.max - self.min
    }

    pub fn contains(&self, x: f64) -> bool {
        self.min <= x && x <= self.max
    }

    pub fn surrounds(&self, x: f64) -> bool {
        self.min < x && x < self.max
    }

    pub fn clamp(&self, x: f64) -> f64 {
        if x < self.min {
            self.min
        } else if x > self.max {
            self.max
        } else {
            x
        }
    }

    pub fn random(&self, rng: &mut dyn RngCore) -> f64 {
        rng.random_range(self.min..self.max)
    }

    pub fn expand(&self, padding: f64) -> Self {
        let padding = padding / 2.;
        Self::new(self.min - padding, self.max + padding)
    }

    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let t0 = self.min.max(other.min);
        let t1 = self.max.min(other.max);
        if t0 < t1 {
            Some(Self::new(t0, t1))
        } else {
            None
        }
    }

    pub fn union(&self, other: &Self) -> Self {
        let min = self.min.min(other.min);
        let max = self.max.max(other.max);
        Self::new(min, max)
    }
}

impl Default for Interval {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Add<f64> for Interval {
    type Output = Self;

    fn add(self, rhs: f64) -> Self::Output {
        Interval::new(self.min + rhs, self.max + rhs)
    }
}

impl Add<Interval> for f64 {
    type Output = Interval;

    fn add(self, rhs: Interval) -> Self::Output {
        Interval::new(self + rhs.min, self + rhs.max)
    }
}