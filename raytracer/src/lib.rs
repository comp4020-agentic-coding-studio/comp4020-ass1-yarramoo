pub mod colour;
pub mod ray;
pub mod hittable;
pub mod interval;
pub mod camera;
pub mod material;
pub mod aabb;
pub mod bvh;
pub mod texture;
pub mod rtw_image;
pub mod perlin;
pub mod demo_scenes;
pub mod onb;
pub mod pdf;

pub mod vector {
    use nalgebra::{Vector2, Vector3};
    use rand::{Rng, RngCore};

    pub type V3 = Vector3<f64>;
    pub type V2 = Vector2<f64>;

    pub fn random_vector(rng: &mut dyn RngCore) -> V3 {
        V3::new(
            rng.random(),
            rng.random(),
            rng.random()
        )
    }

    pub fn random_vector_range(rng: &mut dyn RngCore, min: f64, max: f64) -> V3 {
        V3::new(
            rng.random_range(min..max),
            rng.random_range(min..max),
            rng.random_range(min..max),
        )
    }

    pub fn random_unit_vector(rng: &mut dyn RngCore) -> V3 {
        loop {
            let p = random_vector_range(rng, -1., 1.);
            let length = p.norm_squared();
            if 1e-160 < length && length <= 1. {
                return p / length.sqrt()
            }
        }
    }

    pub fn random_on_hemisphere(rng: &mut dyn RngCore, normal: V3) -> V3 {
        let v = random_unit_vector(rng);
        if v.dot(&normal) > 0. {
            v
        } else {
            -v
        }
    }

    pub fn random_in_unit_disk(rng: &mut dyn RngCore) -> V3 {
        loop {
            let p = V3::new(rng.random_range(-1.0..1.0), rng.random_range(-1.0..1.0), 0.);
            if p.norm() < 1. {
                return p;
            }
        }
    }

    pub fn near_zero(v: V3) -> bool {
        let s = 1e-8;
        v.x.abs() < s && v.y.abs() < s && v.z.abs() < s
    }

    pub fn reflect(v: V3, n: V3) -> V3 {
        v - 2.*v.dot(&n)*n
    }

    pub fn refract(uv: V3, n: V3, etai_over_etat: f64) -> V3 {
        let cos_theta = n.dot(&-uv).min(1.);
        let r_out_perp = etai_over_etat * (uv + cos_theta*n);
        let r_out_parallel = - (1. - r_out_perp.norm_squared()).abs().sqrt() * n;
        r_out_perp + r_out_parallel
    }
}


pub mod sample {
    use rand::RngCore;

    use crate::{vector::V3, interval::Interval};

    pub trait Sample {
        fn sample(rng: &mut dyn RngCore) -> V3;
    }

    pub struct Square;
    impl Sample for Square {
        fn sample(rng: &mut dyn RngCore) -> V3 {
            V3::new(
                Interval::new(-0.5, 0.5).random(rng),
                Interval::new(-0.5, 0.5).random(rng),
                0.
            )
        }
    } 
}

#[macro_export]
macro_rules! arc_dyn {
    ($hittable:expr, $t:path) => {
        std::sync::Arc::new($hittable) as std::sync::Arc<dyn $t>   
    };
}

#[macro_export]
macro_rules! make_material {
    (Lambertian($r:expr, $g:expr, $b:expr)) => {
        std::sync::Arc::new(Lambertian::from_rgb($r, $g, $b)) as std::sync::Arc<dyn Material>
    };
    (Lambertian($texture:expr)) => {
        std::sync::Arc::new(Lambertian::new($texture)) as std::sync::Arc<dyn Material>
    };
    (DiffuseLight($r:expr, $g:expr, $b:expr)) => {
        std::sync::Arc::new(DiffuseLight::from_colour(Colour::new($r, $g, $g))) as std::sync::Arc<dyn Material>
    };
    (Dialectric $refr:expr) => {
        std::sync::Arc::new(Dialectric::new($refr)) as std::sync::Arc<dyn Material>
    };
    (Metal ($r:expr, $g:expr, $b:expr), $fuzz:expr) => {
        std::sync::Arc::new(Metal::new(Colour::new($r, $g, $b), $fuzz)) as std::sync::Arc<dyn Material>
    }
}

#[macro_export]
macro_rules! world_add {
    (Quad $world:expr, ($Qx:expr,$Qy:expr,$Qz:expr), ($vx:expr,$vy:expr,$vz:expr), ($ux:expr,$uy:expr,$uz:expr), $mat:expr) => {
        $world.add(Box::new(Quad::new(
            V3::new($Qx, $Qy, $Qz),
            V3::new($vx, $vy, $vz),
            V3::new($ux, $uy, $uz),
            $mat
        )))
    };
    (Sphere $world:expr, ($x:expr,$y:expr,$z:expr), $radius:expr, $mat:expr) => {
        $world.add(Box::new(Sphere::new(
            V3::new($x, $y, $z),
            $radius,
            $mat
        )))
    };
}