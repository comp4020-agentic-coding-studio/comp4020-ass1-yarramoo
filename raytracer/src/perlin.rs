use rand::{rng, seq::SliceRandom};

use crate::vector::{V3, random_unit_vector};


#[derive(Debug)]
pub struct Perlin {
    rand_vec: Vec<V3>,
    perm_x: Vec<u16>,
    perm_y: Vec<u16>,
    perm_z: Vec<u16>
}

impl Perlin {
    const POINT_COUNT: u16 = 256;

    pub fn new() -> Self {
        let mut rng = rng();

        let mut rand_vec = Vec::with_capacity(Self::POINT_COUNT as usize);
        for _ in 0..Self::POINT_COUNT {
            rand_vec.push(random_unit_vector(&mut rng));
        }

        let mut gen_permutation = || { 
            let mut tmp: Vec<_> = (0..Self::POINT_COUNT).collect();
            tmp.shuffle(&mut rng);
            tmp
        };

        let perm_x = gen_permutation();
        let perm_y = gen_permutation();
        let perm_z = gen_permutation();

        Self { rand_vec, perm_x, perm_y, perm_z }
    }

    pub fn noise(&self, point: V3) -> f64 {
        let u = point.x - point.x.floor();
        let v = point.y - point.y.floor();
        let w = point.z - point.z.floor();

        let i = point.x.floor() as isize;
        let j = point.y.floor() as isize;
        let k = point.z.floor() as isize;

        let mut c = [[[V3::default(); 2]; 2]; 2];

        for di in 0..2isize {
            for dj in 0..2isize {
                for dk in 0..2isize {
                    c[di as usize][dj as usize][dk as usize] = self.rand_vec[
                        (self.perm_x[((i + di) & 255) as usize] ^
                        self.perm_y[((j + dj) & 255) as usize] ^
                        self.perm_z[((k + dk) & 255) as usize]) as usize
                    ]
                }
            }
        }

        Self::perlin_interp(c, u, v, w)
    }

    pub fn turb(&self, point: V3, depth: usize) -> f64 {
        let mut acc = 0.;
        let mut temp_p = point;
        let mut weight = 1.;

        for _ in 0..depth {
            acc += weight * self.noise(temp_p);
            weight *= 0.5;
            temp_p *= 2.;
        }

        acc.abs()
    }

    fn perlin_interp(c: [[[V3;2];2];2], u: f64, v: f64, w: f64) -> f64 {
        let uu = u*u*(3.-2.*u);
        let vv = v*v*(3.-2.*v);
        let ww = w*w*(3.-2.*w);
        let mut acc = 0.0;

        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    let fi = i as f64;
                    let fj = j as f64;
                    let fk = k as f64;
                    let weight_v = V3::new(u-fi, v-fj, w-fk);
                    acc += (fi*uu + (1.-fi)*(1.-uu))
                         * (fj*vv + (1.-fj)*(1.-vv))
                         * (fk*ww + (1.-fk)*(1.-ww))
                         * c[i][j][k].dot(&weight_v);
                }
            }
        }

        acc
    }

    fn _trilinear_interp(c: [[[f64;2];2];2], u: f64, v: f64, w: f64) -> f64 {
        let mut acc = 0.;
        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    let fi = i as f64;
                    let fj = j as f64;
                    let fk = k as f64;
                    // skip bounds check in index
                    let c_val = unsafe { c.get_unchecked(i).get_unchecked(j).get_unchecked(k) };
                    acc += (fi*u + (1.-fi)*(1.-u))
                         * (fj*v + (1.-fj)*(1.-v))
                         * (fk*w + (1.-fk)*(1.-w))
                         * c_val
                }
            }
        }

        acc
    }
}

impl Default for Perlin {
    fn default() -> Self {
        Self::new()
    }
}