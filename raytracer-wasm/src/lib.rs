use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use wasm_bindgen::prelude::*;

use raytracer::camera::{Camera, CameraBuilder};
use raytracer::colour::{linear_to_gamma, Colour};
use raytracer::hittable::{sphere::Sphere, HittableList};
use raytracer::interval::Interval;
use raytracer::material::{dielectric::Dialectric, lambertian::Lambertian, metal::Metal, Material};
use raytracer::vector::V3;

const MAX_DEPTH: usize = 8;
const ORBIT_EPS: f64 = 0.05;

/// Mirrors a `MaterialKind` const enum on the TS side.
const KIND_METAL: u32 = 1;
const KIND_DIALECTRIC: u32 = 2;

struct SphereDesc {
    centre: V3,
    radius: f64,
    material: Arc<dyn Material>,
}

fn default_spheres() -> Vec<SphereDesc> {
    vec![
        SphereDesc {
            centre: V3::new(0.0, -100.5, -1.0),
            radius: 100.0,
            material: Arc::new(Lambertian::from_rgb(0.5, 0.5, 0.5)),
        },
        SphereDesc {
            centre: V3::new(0.0, 0.0, -1.2),
            radius: 0.5,
            material: Arc::new(Lambertian::from_rgb(0.1, 0.2, 0.5)),
        },
        SphereDesc {
            centre: V3::new(-1.0, 0.0, -1.0),
            radius: 0.5,
            material: Arc::new(Dialectric::new(1.5)),
        },
        SphereDesc {
            centre: V3::new(1.0, 0.0, -1.0),
            radius: 0.5,
            material: Arc::new(Metal::new(Colour::new(0.8, 0.6, 0.2), 0.0)),
        },
    ]
}

fn build_world(spheres: &[SphereDesc]) -> HittableList {
    let mut world = HittableList::new();
    for s in spheres {
        world.add(Box::new(Sphere::new(s.centre, s.radius, &s.material)));
    }
    world
}

fn make_material(kind: u32, r: f64, g: f64, b: f64, param: f64) -> Arc<dyn Material> {
    match kind {
        KIND_METAL => Arc::new(Metal::new(Colour::new(r, g, b), param)),
        KIND_DIALECTRIC => Arc::new(Dialectric::new(param)),
        _ => Arc::new(Lambertian::from_rgb(r, g, b)),
    }
}

fn build_camera(width: u32, height: u32, vfov: f64, lookat: V3, theta: f64, phi: f64, radius: f64) -> Camera {
    let lookfrom = lookat
        + radius
            * V3::new(
                phi.cos() * theta.sin(),
                theta.cos(),
                phi.sin() * theta.sin(),
            );
    CameraBuilder::new()
        .with_width(width as usize)
        .with_height(height as usize)
        .with_vfov(vfov)
        .with_camera_pos(lookfrom, lookat, V3::new(0.0, 1.0, 0.0))
        .build()
        .expect("width and height are always set")
}

#[wasm_bindgen]
pub struct Scene {
    spheres: Vec<SphereDesc>,
    world: HittableList,
    camera: Camera,
    lookat: V3,
    theta: f64,
    phi: f64,
    orbit_radius: f64,
    vfov: f64,
    width: u32,
    height: u32,
    accum: Vec<Colour>,
    sample_count: u32,
}

#[wasm_bindgen]
impl Scene {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Scene {
        console_error_panic_hook::set_once();
        let lookat = V3::new(0.0, 0.0, -1.0);
        let (theta, phi, orbit_radius, vfov) = (FRAC_PI_2, 0.0, 4.0, 40.0);
        let spheres = default_spheres();
        let world = build_world(&spheres);
        let camera = build_camera(width, height, vfov, lookat, theta, phi, orbit_radius);
        Scene {
            spheres,
            world,
            camera,
            lookat,
            theta,
            phi,
            orbit_radius,
            vfov,
            width,
            height,
            accum: vec![Colour::default(); (width * height) as usize],
            sample_count: 0,
        }
    }

    /// kind: 0 = Lambertian (param unused), 1 = Metal (param = fuzz 0..1),
    /// 2 = Dialectric (param = refraction index).
    pub fn set_sphere_material(&mut self, index: u32, kind: u32, r: f64, g: f64, b: f64, param: f64) {
        if let Some(desc) = self.spheres.get_mut(index as usize) {
            desc.material = make_material(kind, r, g, b, param);
            self.world = build_world(&self.spheres);
            self.reset_accumulation();
        }
    }

    pub fn orbit_camera(&mut self, d_theta: f64, d_phi: f64) {
        self.theta = (self.theta + d_theta).clamp(ORBIT_EPS, PI - ORBIT_EPS);
        self.phi += d_phi;
        self.camera = build_camera(self.width, self.height, self.vfov, self.lookat, self.theta, self.phi, self.orbit_radius);
        self.reset_accumulation();
    }

    pub fn zoom(&mut self, delta: f64) {
        self.orbit_radius = (self.orbit_radius + delta).clamp(1.5, 20.0);
        self.camera = build_camera(self.width, self.height, self.vfov, self.lookat, self.theta, self.phi, self.orbit_radius);
        self.reset_accumulation();
    }

    /// Adds `samples_this_pass` more samples per pixel to the running average.
    /// Reuses `Camera::get_ray`/`Camera::ray_colour` directly — no duplicated
    /// tracing logic between the CLI and the wasm build.
    pub fn render_pass(&mut self, samples_this_pass: u32) {
        let mut rng = rand::rng();
        for j in 0..self.height as usize {
            for i in 0..self.width as usize {
                let mut colour = Colour::default();
                for _ in 0..samples_this_pass {
                    let ray = self.camera.get_ray(i, j, &mut rng);
                    colour += self.camera.ray_colour(&ray, &self.world, MAX_DEPTH, &mut rng);
                }
                self.accum[j * self.width as usize + i] += colour;
            }
        }
        self.sample_count += samples_this_pass;
    }

    /// Gamma-corrected, clamped RGBA — feed straight into `new ImageData(...)`.
    pub fn pixels(&self) -> Vec<u8> {
        let scale = 1.0 / self.sample_count.max(1) as f64;
        let clamp01 = Interval::new(0.0, 0.999);
        let mut out = Vec::with_capacity(self.accum.len() * 4);
        for c in &self.accum {
            let avg = c * scale;
            out.push((clamp01.clamp(linear_to_gamma(avg.x)) * 256.0) as u8);
            out.push((clamp01.clamp(linear_to_gamma(avg.y)) * 256.0) as u8);
            out.push((clamp01.clamp(linear_to_gamma(avg.z)) * 256.0) as u8);
            out.push(255);
        }
        out
    }

    pub fn sample_count(&self) -> u32 {
        self.sample_count
    }

    fn reset_accumulation(&mut self) {
        self.accum.iter_mut().for_each(|c| *c = Colour::default());
        self.sample_count = 0;
    }
}
