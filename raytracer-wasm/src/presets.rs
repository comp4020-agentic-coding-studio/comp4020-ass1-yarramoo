//! Preset scenes for the "load a different scene" dropdown. Each function
//! builds a `HittableList` plus the camera framing to view it, reusing
//! `raytracer` primitives directly the same way `raytracer::demo_scenes`
//! does natively. None of these need new ray-tracer physics beyond the
//! `DispersiveGlass` material and the defocus-angle exposure added alongside
//! this module — they're just new arrangements of existing hittables.

use std::f64::consts::FRAC_PI_2;
use std::sync::Arc;

use raytracer::colour::Colour;
use raytracer::hittable::medium::ConstantMedium;
use raytracer::hittable::quad::{quad_box, Quad};
use raytracer::hittable::sphere::Sphere;
use raytracer::hittable::{Hittable, HittableList};
use raytracer::material::dielectric::Dialectric;
use raytracer::material::diffuse_light::DiffuseLight;
use raytracer::material::dispersive_glass::DispersiveGlass;
use raytracer::material::lambertian::Lambertian;
use raytracer::material::metal::Metal;
use raytracer::material::Material;
use raytracer::vector::V3;

use crate::SceneSetup;

fn mat<M: Material + 'static>(m: M) -> Arc<dyn Material> {
    Arc::new(m)
}

/// A five-sided box (everything but the face at `x = +half`, where the
/// camera sits) — floor, ceiling, back wall and two coloured side walls.
/// Shared by the Cornell box and foggy room presets, which differ only in
/// what they put inside it.
fn room(half: f64) -> HittableList {
    let mut world = HittableList::new();
    let white = mat(Lambertian::from_rgb(0.73, 0.73, 0.73));
    let red = mat(Lambertian::from_rgb(0.65, 0.05, 0.05));
    let green = mat(Lambertian::from_rgb(0.12, 0.45, 0.15));
    let span = 2.0 * half;

    // Back wall (x = -half).
    world.add(Box::new(Quad::new(V3::new(-half, -half, -half), V3::new(0., 0., span), V3::new(0., span, 0.), &white)));
    // Left wall (z = -half).
    world.add(Box::new(Quad::new(V3::new(-half, -half, -half), V3::new(span, 0., 0.), V3::new(0., span, 0.), &red)));
    // Right wall (z = +half).
    world.add(Box::new(Quad::new(V3::new(-half, -half, half), V3::new(span, 0., 0.), V3::new(0., span, 0.), &green)));
    // Floor (y = -half).
    world.add(Box::new(Quad::new(V3::new(-half, -half, -half), V3::new(span, 0., 0.), V3::new(0., 0., span), &white)));
    // Ceiling (y = +half).
    world.add(Box::new(Quad::new(V3::new(-half, half, -half), V3::new(span, 0., 0.), V3::new(0., 0., span), &white)));

    world
}

/// A quad box, room, and a recessed ceiling light — the terminationKind=2
/// ("emitted") case of the trace format is otherwise unreachable, since the
/// classic three-sphere scene has no lights at all. Click a lit patch of
/// floor versus a shadowed corner and the dot's colour differs without
/// reading any label.
pub fn cornell_box() -> SceneSetup {
    let half = 2.0;
    let mut world = room(half);

    let light = mat(DiffuseLight::from_colour(Colour::new(15., 15., 15.)));
    let light_half = 0.6;
    let light_q = V3::new(-light_half, half - 0.02, -light_half);
    let light_u = V3::new(2. * light_half, 0., 0.);
    let light_v = V3::new(0., 0., 2. * light_half);
    world.add(Box::new(Quad::new(light_q, light_u, light_v, &light)));
    // Duplicated (not shared) geometry: boxed into `world` to be hit like any
    // other object, Arc'd into `lights` so `Camera::ray_colour`'s mixture PDF
    // can aim bounces at it directly (next-event estimation).
    let lights: Vec<Arc<dyn Hittable>> = vec![Arc::new(Quad::new(light_q, light_u, light_v, &light))];

    let metal = mat(Metal::new(Colour::new(0.8, 0.75, 0.6), 0.05));
    world.add(Box::new(Sphere::new(V3::new(0.9, -half + 0.7, 0.5), 0.7, &metal)));

    let glass = mat(Dialectric::new(1.5));
    world.add(Box::new(Sphere::new(V3::new(-0.8, -half + 0.6, -0.6), 0.6, &glass)));

    SceneSetup {
        spheres: Vec::new(),
        world,
        lights,
        lookat: V3::new(0., 0., 0.),
        theta: FRAC_PI_2,
        phi: 0.0,
        orbit_radius: 5.0,
        vfov: 40.0,
        defocus_angle: 0.0,
        background: Colour::new(0., 0., 0.),
        max_depth: 16,
    }
}

/// Same room shell, but the interior is mostly a dense `ConstantMedium` fog
/// block instead of solid objects. Spotlights volumetric scattering: with
/// the preset's recommended higher max depth, a path traced into the fog
/// visibly jitters through many short isotropic bounces instead of the
/// clean single surface hit every other preset shows.
pub fn foggy_room() -> SceneSetup {
    let half = 2.2;
    let mut world = room(half);
    let white = mat(Lambertian::from_rgb(0.73, 0.73, 0.73));

    let light = mat(DiffuseLight::from_colour(Colour::new(10., 10., 10.)));
    let light_half = 0.7;
    let light_q = V3::new(-light_half, half - 0.02, -light_half);
    let light_u = V3::new(2. * light_half, 0., 0.);
    let light_v = V3::new(0., 0., 2. * light_half);
    world.add(Box::new(Quad::new(light_q, light_u, light_v, &light)));
    let lights: Vec<Arc<dyn Hittable>> = vec![Arc::new(Quad::new(light_q, light_u, light_v, &light))];

    let fog_half = half - 0.3;
    let boundary: Arc<dyn Hittable> = Arc::new(quad_box(
        V3::new(-fog_half, -fog_half, -fog_half),
        V3::new(fog_half, fog_half, fog_half),
        &white,
    ));
    world.add(Box::new(ConstantMedium::from_albedo(&boundary, 1.8, Colour::new(0.9, 0.9, 0.95))));

    SceneSetup {
        spheres: Vec::new(),
        world,
        lights,
        lookat: V3::new(0., 0., 0.),
        theta: FRAC_PI_2,
        phi: 0.0,
        orbit_radius: 5.0,
        vfov: 40.0,
        defocus_angle: 0.0,
        background: Colour::new(0., 0., 0.),
        max_depth: 40,
    }
}

/// A `DispersiveGlass` sphere backlit by a bright emissive wall. Clicking
/// through the glass splits the traced path into three colour-tagged
/// sub-paths (red/green/blue) that visibly fan out before hitting the
/// backdrop — chromatic dispersion, made deliberately dramatic (see the
/// comment on `DispersiveGlass::from_spread` below).
pub fn dispersive_prism() -> SceneSetup {
    let mut world = HittableList::new();

    let ground = mat(Lambertian::from_rgb(0.4, 0.4, 0.42));
    world.add(Box::new(Sphere::new(V3::new(0., -100.6, 0.), 100., &ground)));

    let backdrop = mat(DiffuseLight::from_colour(Colour::new(6., 6., 6.)));
    world.add(Box::new(Quad::new(
        V3::new(-1.2, -1.0, -1.5),
        V3::new(0., 2.6, 0.),
        V3::new(0., 0., 3.0),
        &backdrop,
    )));

    // Real glass separates red/blue by an index-of-refraction difference of
    // roughly 0.01 — invisible at this scale. Exaggerated ~15x here so the
    // split reads clearly on a 480x300 canvas instead of needing a
    // spectrometer to notice.
    let glass = mat(DispersiveGlass::from_spread(1.5, 0.15));
    world.add(Box::new(Sphere::new(V3::new(0., 0.1, 0.), 0.7, &glass)));

    SceneSetup {
        spheres: Vec::new(),
        world,
        lights: Vec::new(),
        lookat: V3::new(0., 0.1, 0.),
        theta: FRAC_PI_2,
        phi: 0.0,
        orbit_radius: 3.2,
        vfov: 35.0,
        defocus_angle: 0.0,
        background: Colour::new(0.02, 0.02, 0.03),
        max_depth: 16,
    }
}

/// A diagonal row of spheres receding from the camera, with `defocus_angle`
/// preset on by default and `focus_distance` (set from `orbit_radius` in
/// `Scene::apply_setup`) landing on the middle sphere — it renders sharp
/// while the near/far ones blur, and each bundle of traced rays for a single
/// clicked pixel visibly fans out across the lens instead of converging to
/// one line, which *is* the depth-of-field explanation.
pub fn depth_of_field() -> SceneSetup {
    let mut world = HittableList::new();

    let ground = mat(Lambertian::from_rgb(0.5, 0.5, 0.5));
    world.add(Box::new(Sphere::new(V3::new(0., -100.5, 0.), 100., &ground)));

    let placements: [(f64, f64, (f64, f64, f64)); 5] = [
        (-3.0, -1.2, (0.8, 0.2, 0.2)),
        (-1.5, -0.6, (0.9, 0.7, 0.2)),
        (0.0, 0.0, (0.85, 0.85, 0.9)),
        (1.5, 0.6, (0.2, 0.7, 0.3)),
        (3.0, 1.2, (0.3, 0.4, 0.9)),
    ];
    for (x, z, (r, g, b)) in placements {
        let material = if (x, z) == (0.0, 0.0) {
            mat(Metal::new(Colour::new(r, g, b), 0.02))
        } else {
            mat(Lambertian::from_rgb(r, g, b))
        };
        world.add(Box::new(Sphere::new(V3::new(x, 0.0, z), 0.5, &material)));
    }

    SceneSetup {
        spheres: Vec::new(),
        world,
        lights: Vec::new(),
        lookat: V3::new(0., 0., 0.),
        theta: FRAC_PI_2,
        phi: 0.0,
        orbit_radius: 4.0,
        vfov: 40.0,
        defocus_angle: 1.2,
        background: Colour::new(0.7, 0.8, 1.0),
        max_depth: 12,
    }
}
