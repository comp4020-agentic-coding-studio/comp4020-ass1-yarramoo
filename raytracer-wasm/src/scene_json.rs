//! Parses the "Custom (JSON)" scene textarea into a `HittableList` — the
//! "author your own scene" answer to "like the ray tracing project from the
//! beginning". Deliberately narrower than the full native scene format:
//! spheres, quads, and the five material kinds the classic/preset scenes
//! already use. Validation errors carry `serde_json`'s own line/column
//! message straight through to the UI.

use std::sync::Arc;
use std::f64::consts::FRAC_PI_2;

use raytracer::colour::Colour;
use raytracer::hittable::quad::Quad;
use raytracer::hittable::sphere::Sphere;
use raytracer::hittable::{Hittable, HittableList};
use raytracer::material::dielectric::Dialectric;
use raytracer::material::diffuse_light::DiffuseLight;
use raytracer::material::dispersive_glass::DispersiveGlass;
use raytracer::material::lambertian::Lambertian;
use raytracer::material::metal::Metal;
use raytracer::material::Material;
use raytracer::vector::V3;
use serde::Deserialize;

use crate::SceneSetup;

#[derive(Deserialize)]
struct SceneFile {
    objects: Vec<ObjectDesc>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ObjectDesc {
    Sphere { centre: [f64; 3], radius: f64, material: MaterialDesc },
    Quad { q: [f64; 3], u: [f64; 3], v: [f64; 3], material: MaterialDesc },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum MaterialDesc {
    Lambertian { albedo: [f64; 3] },
    Metal { albedo: [f64; 3], fuzz: f64 },
    Dielectric { ior: f64 },
    DiffuseLight { colour: [f64; 3] },
    Dispersive { ior: f64, spread: f64 },
}

fn to_v3(a: [f64; 3]) -> V3 {
    V3::new(a[0], a[1], a[2])
}

fn build_material(desc: &MaterialDesc) -> Arc<dyn Material> {
    match desc {
        MaterialDesc::Lambertian { albedo } => {
            let [r, g, b] = *albedo;
            Arc::new(Lambertian::from_rgb(r, g, b))
        }
        MaterialDesc::Metal { albedo, fuzz } => {
            let [r, g, b] = *albedo;
            Arc::new(Metal::new(Colour::new(r, g, b), *fuzz))
        }
        MaterialDesc::Dielectric { ior } => Arc::new(Dialectric::new(*ior)),
        MaterialDesc::DiffuseLight { colour } => {
            let [r, g, b] = *colour;
            Arc::new(DiffuseLight::from_colour(Colour::new(r, g, b)))
        }
        MaterialDesc::Dispersive { ior, spread } => Arc::new(DispersiveGlass::from_spread(*ior, *spread)),
    }
}

/// Builds a scene from JSON text, or a human-readable error naming what was
/// wrong — `serde_json`'s errors already carry a line/column, which reads
/// as `"expected value at line 3 column 12"` in the UI without any
/// translation on this side.
pub fn build_from_json(json: &str) -> Result<SceneSetup, String> {
    let scene_file: SceneFile = serde_json::from_str(json).map_err(|e| e.to_string())?;
    if scene_file.objects.is_empty() {
        return Err("scene must contain at least one object".to_string());
    }

    let mut world = HittableList::new();
    // Duplicated (not shared) geometry: a Quad is cheap and stateless, so a
    // DiffuseLight quad gets built twice — once boxed into `world` to be hit
    // like any other object, once Arc'd into `lights` so it can be sampled
    // directly by the mixture PDF (see `Camera::ray_colour`).
    let mut lights: Vec<Arc<dyn Hittable>> = Vec::new();
    for obj in &scene_file.objects {
        match obj {
            ObjectDesc::Sphere { centre, radius, material } => {
                if *radius <= 0.0 {
                    return Err("sphere radius must be positive".to_string());
                }
                let m = build_material(material);
                world.add(Box::new(Sphere::new(to_v3(*centre), *radius, &m)));
            }
            ObjectDesc::Quad { q, u, v, material } => {
                let m = build_material(material);
                world.add(Box::new(Quad::new(to_v3(*q), to_v3(*u), to_v3(*v), &m)));
                if matches!(material, MaterialDesc::DiffuseLight { .. }) {
                    lights.push(Arc::new(Quad::new(to_v3(*q), to_v3(*u), to_v3(*v), &m)));
                }
            }
        }
    }

    let bbox = world.bounding_box();
    let lookat = V3::new(
        (bbox.x.min + bbox.x.max) / 2.,
        (bbox.y.min + bbox.y.max) / 2.,
        (bbox.z.min + bbox.z.max) / 2.,
    );
    let diagonal = (bbox.x.size().powi(2) + bbox.y.size().powi(2) + bbox.z.size().powi(2)).sqrt();
    let orbit_radius = (diagonal * 1.3).clamp(1.5, 60.0);

    Ok(SceneSetup {
        spheres: Vec::new(),
        world,
        lights,
        lookat,
        theta: FRAC_PI_2,
        phi: 0.0,
        orbit_radius,
        vfov: 40.0,
        defocus_angle: 0.0,
        background: Colour::new(0.7, 0.8, 1.0),
        max_depth: 8,
    })
}
