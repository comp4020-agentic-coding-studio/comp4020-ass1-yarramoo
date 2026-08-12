use std::f64::consts::FRAC_PI_2;
use std::sync::Arc;

use wasm_bindgen::prelude::*;

use raytracer::camera::{Camera, CameraBuilder};
use raytracer::colour::{linear_to_gamma, Colour};
use raytracer::hittable::{sphere::Sphere, Hittable, HittableList};
use raytracer::interval::Interval;
use raytracer::material::{dielectric::Dialectric, lambertian::Lambertian, metal::Metal, Material};
use raytracer::vector::V3;

const ORBIT_EPS: f64 = 0.05;
// The ground is a single giant sphere (radius 100, centred far below the
// scene) rather than a true infinite plane, so it curves away from the
// camera's orbit axis. Letting theta reach FRAC_PI_2 puts the camera right
// at that curved surface — a small overshoot embeds it in the sphere and
// every ray starts inside solid geometry, rendering solid black. Capping
// theta comfortably above the horizon keeps the camera clear across the
// full zoom range.
const MAX_THETA: f64 = FRAC_PI_2 - 0.15;

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
    max_depth: u32,
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
            max_depth: 8,
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

    pub fn set_max_depth(&mut self, depth: u32) {
        self.max_depth = depth.max(1); // depth=0 would render solid black via ray_colour's early return
        self.reset_accumulation();
    }

    pub fn orbit_camera(&mut self, d_theta: f64, d_phi: f64) {
        self.theta = (self.theta + d_theta).clamp(ORBIT_EPS, MAX_THETA);
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
                    colour += self.camera.ray_colour(&ray, &self.world, self.max_depth as usize, &mut rng);
                }
                self.accum[j * self.width as usize + i] += colour;
            }
        }
        self.sample_count += samples_this_pass;
    }

    /// Renders `samples` fresh samples per pixel into a local buffer and
    /// returns it gamma-corrected, exactly like `pixels()` — but never
    /// touches `self.accum`/`self.sample_count`, so it doesn't disturb the
    /// live progressive render. Used to snapshot "1 sample" for comparison
    /// against however far the live render has already converged.
    pub fn render_snapshot(&self, samples: u32) -> Vec<u8> {
        let mut rng = rand::rng();
        let samples = samples.max(1);
        let mut local = vec![Colour::default(); (self.width * self.height) as usize];
        for j in 0..self.height as usize {
            for i in 0..self.width as usize {
                let mut colour = Colour::default();
                for _ in 0..samples {
                    let ray = self.camera.get_ray(i, j, &mut rng);
                    colour += self.camera.ray_colour(&ray, &self.world, self.max_depth as usize, &mut rng);
                }
                local[j * self.width as usize + i] = colour;
            }
        }

        let scale = 1.0 / samples as f64;
        let clamp01 = Interval::new(0.0, 0.999);
        let mut out = Vec::with_capacity(local.len() * 4);
        for c in &local {
            let avg = c * scale;
            out.push((clamp01.clamp(linear_to_gamma(avg.x)) * 256.0) as u8);
            out.push((clamp01.clamp(linear_to_gamma(avg.y)) * 256.0) as u8);
            out.push((clamp01.clamp(linear_to_gamma(avg.z)) * 256.0) as u8);
            out.push(255);
        }
        out
    }

    /// Traces the ray through pixel (i, j), following its bounce path up to
    /// `max_depth` vertices, and projects each 3D vertex back onto the
    /// camera's own pixel grid. Returns a flat `[x0, y0, x1, y1, ...]` list
    /// of *pixel-space* coordinates the caller can draw as a polyline
    /// directly over the canvas — vertex 0 is the clicked (i, j) itself, and
    /// the last vertex is either a surface hit that absorbed the ray or an
    /// escape point pushed out along the final ray direction.
    ///
    /// A vertex whose `project_point` fails (behind the camera, or exactly
    /// parallel to the image plane) truncates the path there rather than
    /// drawing garbage — this can genuinely happen for a bounce that heads
    /// back past the camera.
    pub fn trace_pixel(&self, i: u32, j: u32, max_depth: u32) -> Vec<f64> {
        let mut rng = rand::rng();
        let mut current = self.camera.get_ray(i as usize, j as usize, &mut rng);
        let mut points_3d: Vec<V3> = Vec::new();
        const ESCAPE_DISTANCE: f64 = 50.0;
        for _ in 0..max_depth.max(1) {
            match self.world.hit(&current, Interval::new(0.001, f64::INFINITY), &mut rng) {
                None => {
                    // Not unit length off a primary/Lambertian ray, so
                    // normalize before extending — otherwise the escape
                    // segment's length varies wildly by which bounce escaped.
                    points_3d.push(current.origin + ESCAPE_DISTANCE * current.direction.normalize());
                    break;
                }
                Some(hr) => {
                    points_3d.push(hr.point);
                    match hr.material.scatter(&current, &hr, &mut rng) {
                        None => break, // absorbed
                        Some((next_ray, _attenuation)) => current = next_ray,
                    }
                }
            }
        }

        let mut out = Vec::with_capacity((points_3d.len() + 1) * 2);
        out.push(i as f64);
        out.push(j as f64);
        for p in &points_3d {
            match self.camera.project_point(*p) {
                Some((x, y)) => {
                    out.push(x);
                    out.push(y);
                }
                None => break,
            }
        }
        out
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
