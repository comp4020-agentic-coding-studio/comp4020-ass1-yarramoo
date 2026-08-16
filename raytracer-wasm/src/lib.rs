use std::f64::consts::FRAC_PI_2;
use std::sync::Arc;

use rand::Rng;
use wasm_bindgen::prelude::*;

use raytracer::camera::{Camera, CameraBuilder, SamplingStrategy};
use raytracer::colour::{linear_to_gamma, Colour};
use raytracer::hittable::{sphere::Sphere, Hittable, HittableList};
use raytracer::interval::Interval;
use raytracer::material::{dielectric::Dialectric, lambertian::Lambertian, metal::Metal, Material};
use raytracer::pdf::{CosinePdf, HittablePdf, Pdf, UniformHemispherePdf};
use raytracer::ray::Ray;
use raytracer::vector::V3;

mod presets;
mod scene_json;

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

fn build_camera(
    width: u32,
    height: u32,
    vfov: f64,
    lookat: V3,
    theta: f64,
    phi: f64,
    radius: f64,
    defocus_angle: f64,
    background: Colour,
) -> Camera {
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
        .with_defocus(radius, defocus_angle)
        .with_background(background)
        .build()
        .expect("width and height are always set")
}

/// Everything a preset or a parsed custom scene needs to hand back to
/// `Scene::apply_setup` in one shot: the world itself, plus every camera
/// framing parameter `Scene` tracks as separate fields (so orbit/zoom keep
/// working against whatever was just loaded) and a recommended starting
/// `max_depth` (fog needs many more bounces than a few glass spheres do).
struct SceneSetup {
    spheres: Vec<SphereDesc>,
    world: HittableList,
    lights: Vec<Arc<dyn Hittable>>,
    lookat: V3,
    theta: f64,
    phi: f64,
    orbit_radius: f64,
    vfov: f64,
    defocus_angle: f64,
    background: Colour,
    max_depth: u32,
}

fn classic_setup() -> SceneSetup {
    let spheres = default_spheres();
    let world = build_world(&spheres);
    SceneSetup {
        spheres,
        world,
        lights: Vec::new(),
        lookat: V3::new(0.0, 0.0, -1.0),
        theta: FRAC_PI_2,
        phi: 0.0,
        orbit_radius: 4.0,
        vfov: 40.0,
        defocus_angle: 0.0,
        background: Colour::new(0.70, 0.80, 1.00),
        max_depth: 8,
    }
}

#[wasm_bindgen]
pub struct Scene {
    spheres: Vec<SphereDesc>,
    world: HittableList,
    lights: Vec<Arc<dyn Hittable>>,
    camera: Camera,
    lookat: V3,
    theta: f64,
    phi: f64,
    orbit_radius: f64,
    vfov: f64,
    defocus_angle: f64,
    background: Colour,
    width: u32,
    height: u32,
    accum: Vec<Colour>,
    sample_count: u32,
    max_depth: u32,
    sampling_strategy: SamplingStrategy,
}

#[wasm_bindgen]
impl Scene {
    #[wasm_bindgen(constructor)]
    pub fn new(width: u32, height: u32) -> Scene {
        console_error_panic_hook::set_once();
        let setup = classic_setup();
        let camera = build_camera(
            width, height, setup.vfov, setup.lookat, setup.theta, setup.phi, setup.orbit_radius,
            setup.defocus_angle, setup.background,
        );
        Scene {
            spheres: setup.spheres,
            world: setup.world,
            lights: setup.lights,
            camera,
            lookat: setup.lookat,
            theta: setup.theta,
            phi: setup.phi,
            orbit_radius: setup.orbit_radius,
            vfov: setup.vfov,
            defocus_angle: setup.defocus_angle,
            background: setup.background,
            width,
            height,
            accum: vec![Colour::default(); (width * height) as usize],
            sample_count: 0,
            max_depth: setup.max_depth,
            sampling_strategy: SamplingStrategy::Naive,
        }
    }

    /// mode: 0 = naive (today's book-1 Lambertian approximation), 1 =
    /// cosine-weighted importance sampling, 2 = mixture (cosine + direct
    /// light sampling, only differs from mode 1 when the scene has lights).
    /// Unknown values fall back to naive. Drives both the progressive render
    /// and `trace_pixel`, so they stay visually consistent with each other.
    pub fn set_sampling_strategy(&mut self, mode: u32) {
        self.sampling_strategy = match mode {
            1 => SamplingStrategy::Cosine,
            2 => SamplingStrategy::Mixture,
            _ => SamplingStrategy::Naive,
        };
        self.reset_accumulation();
    }

    /// kind: 0 = Lambertian (param unused), 1 = Metal (param = fuzz 0..1),
    /// 2 = Dialectric (param = refraction index). Only meaningful for the
    /// classic scene (index into `self.spheres`) — a no-op on any preset or
    /// custom scene, which have no such list.
    pub fn set_sphere_material(&mut self, index: u32, kind: u32, r: f64, g: f64, b: f64, param: f64) {
        if let Some(desc) = self.spheres.get_mut(index as usize) {
            desc.material = make_material(kind, r, g, b, param);
            self.world = build_world(&self.spheres);
            self.reset_accumulation();
        }
    }

    /// Rebuilds the accumulation buffer and camera framing for a new pixel
    /// resolution, keeping the current world/camera-orbit state intact —
    /// called whenever the browser window resizes so the internal render
    /// resolution's aspect ratio tracks the viewport's.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.accum = vec![Colour::default(); (width * height) as usize];
        self.rebuild_camera();
        self.sample_count = 0;
    }

    pub fn set_max_depth(&mut self, depth: u32) {
        self.max_depth = depth.max(1); // depth=0 would render solid black via ray_colour's early return
        self.reset_accumulation();
    }

    /// angle: defocus cone angle in degrees, 0 = pinhole (no blur). Focus
    /// distance always tracks `orbit_radius` — the camera stays focused on
    /// whatever it's orbiting.
    pub fn set_defocus(&mut self, angle: f64) {
        self.defocus_angle = angle.max(0.0);
        self.rebuild_camera();
        self.reset_accumulation();
    }

    /// id: 0 = classic spheres, 1 = Cornell box, 2 = foggy room, 3 =
    /// dispersive prism, 4 = depth of field. Unknown ids fall back to the
    /// classic scene rather than erroring — the dropdown on the JS side is
    /// the only caller and it never sends anything else.
    pub fn load_preset(&mut self, id: u32) {
        let setup = match id {
            1 => presets::cornell_box(),
            2 => presets::foggy_room(),
            3 => presets::dispersive_prism(),
            4 => presets::depth_of_field(),
            _ => classic_setup(),
        };
        self.apply_setup(setup);
    }

    /// Parses and loads a custom JSON scene (see `scene_json` for the
    /// schema). On success, replaces the world and re-frames the camera to
    /// fit it. On failure, leaves the current scene untouched and returns
    /// the parse/validation error for the UI to display.
    pub fn load_json(&mut self, json: &str) -> Result<(), JsValue> {
        let setup = scene_json::build_from_json(json).map_err(|e| JsValue::from_str(&e))?;
        self.apply_setup(setup);
        Ok(())
    }

    pub fn orbit_camera(&mut self, d_theta: f64, d_phi: f64) {
        self.theta = (self.theta + d_theta).clamp(ORBIT_EPS, MAX_THETA);
        self.phi += d_phi;
        self.rebuild_camera();
        self.reset_accumulation();
    }

    pub fn zoom(&mut self, delta: f64) {
        self.orbit_radius = (self.orbit_radius + delta).clamp(1.5, 20.0);
        self.rebuild_camera();
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
                    colour += self.camera.ray_colour(&ray, &self.world, &self.lights, self.sampling_strategy, self.max_depth as usize, &mut rng);
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
                    colour += self.camera.ray_colour(&ray, &self.world, &self.lights, self.sampling_strategy, self.max_depth as usize, &mut rng);
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

    /// Same as `render_snapshot`, but renders with `mode` (same encoding as
    /// `set_sampling_strategy`) instead of `self.sampling_strategy`, and
    /// touches no state at all — not even `self.sampling_strategy` itself.
    /// Exists so the frontend's naive/cosine/mixture comparison grid can
    /// capture one snapshot per strategy without resetting the live
    /// progressive render, which `set_sampling_strategy` always does.
    pub fn render_snapshot_with_strategy(&self, mode: u32, samples: u32) -> Vec<u8> {
        let strategy = match mode {
            1 => SamplingStrategy::Cosine,
            2 => SamplingStrategy::Mixture,
            _ => SamplingStrategy::Naive,
        };
        let mut rng = rand::rng();
        let samples = samples.max(1);
        let mut local = vec![Colour::default(); (self.width * self.height) as usize];
        for j in 0..self.height as usize {
            for i in 0..self.width as usize {
                let mut colour = Colour::default();
                for _ in 0..samples {
                    let ray = self.camera.get_ray(i, j, &mut rng);
                    colour += self.camera.ray_colour(&ray, &self.world, &self.lights, strategy, self.max_depth as usize, &mut rng);
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

    /// Traces the ray(s) through pixel (i, j) and returns every resulting
    /// path in one flat buffer:
    ///
    /// ```text
    /// [num_paths, <path>...]
    /// <path> = [terminationKind, channel, len, x0,y0, x1,y1, ..., x(len-1),y(len-1)]
    /// terminationKind: 0 = escaped, 1 = absorbed (or ran out of bounces),
    ///                  2 = emitted (hit a light)
    /// channel: -1 = neutral, 0/1/2 = R/G/B (only set by dispersive glass)
    /// ```
    ///
    /// A plain scene produces exactly one neutral path — the same shape the
    /// old single-polyline API returned. Two things bundle up more paths:
    ///
    /// - **Dispersion**: a neutral path that hits a `scatter_dispersive`
    ///   material (currently only `DispersiveGlass`) splits into three
    ///   channel-tagged paths, one per colour, each continuing
    ///   independently. A path that already has a channel keeps using that
    ///   channel's deterministic scatter if it hits another dispersive
    ///   surface rather than splitting again, so total paths per
    ///   bundle-ray is capped at 3.
    /// - **Depth of field**: when `defocus_angle > 0`, the pixel is sampled
    ///   several independent times — each through `Camera::get_ray`'s own
    ///   defocus-disk sampling, so each bundle member starts from a
    ///   slightly different point on the lens — instead of once. The
    ///   resulting spread/convergence of those paths *is* the
    ///   depth-of-field visualization.
    ///
    /// Every path's first vertex is the clicked pixel (i, j) itself, in
    /// pixel-space — not the ray's actual 3D origin, which for a defocus
    /// lens sample is off-axis. Subsequent vertices are 3D hit points
    /// projected back onto the pixel grid; a vertex whose `project_point`
    /// fails (behind the camera, or exactly parallel to the image plane —
    /// both happen for real bounce paths) truncates that path there rather
    /// than drawing garbage.
    ///
    /// Alongside each vertex's (x, y) pair, the wire format also carries a
    /// per-vertex PDF-source tag (-1 = not PDF-sampled — the pixel's own
    /// first vertex, a specular bounce, or naive-strategy scattering; 0 =
    /// cosine-weighted; 1 = aimed directly at a light) so the frontend can
    /// colour-code which mechanism produced each bounce under the mixture
    /// strategy. Which of cosine/light fired is decided right here (rather
    /// than delegating to `MixturePdf`, which only exposes the combined
    /// density) specifically so it can be recorded.
    pub fn trace_pixel(&self, i: u32, j: u32, max_depth: u32) -> Vec<f64> {
        const ESCAPE_DISTANCE: f64 = 50.0;
        let max_depth = max_depth.max(1);
        let mut rng = rand::rng();

        struct TracePath {
            points: Vec<V3>,
            sources: Vec<i32>,
            channel: i32,
            termination: u8,
        }

        let bundle_size = if self.defocus_angle > 0.0 { 6 } else { 1 };
        let mut paths: Vec<TracePath> = Vec::new();

        for _ in 0..bundle_size {
            // (ray, channel, points-so-far, sources-so-far, source of `ray`
            // itself) for every sub-path still bouncing. The last element
            // tags the ray currently in flight so it can be recorded
            // alongside whatever point it lands on next.
            let mut active: Vec<(Ray, i32, Vec<V3>, Vec<i32>, i32)> = vec![(
                self.camera.get_ray(i as usize, j as usize, &mut rng),
                -1,
                Vec::new(),
                Vec::new(),
                -1,
            )];

            for _ in 0..max_depth {
                if active.is_empty() {
                    break;
                }
                let mut next_active = Vec::new();
                for (current, channel, mut points, mut sources, ray_source) in active {
                    let Some(hr) = self.world.hit(&current, Interval::new(0.001, f64::INFINITY), &mut rng) else {
                        points.push(current.origin + ESCAPE_DISTANCE * current.direction.normalize());
                        sources.push(ray_source);
                        paths.push(TracePath { points, sources, channel, termination: 0 });
                        continue;
                    };
                    points.push(hr.point);
                    sources.push(ray_source);

                    let emission = hr.material.emitted(hr.texture_coords, hr.point);
                    if emission.x > 0.0 || emission.y > 0.0 || emission.z > 0.0 {
                        paths.push(TracePath { points, sources, channel, termination: 2 });
                        continue;
                    }

                    if channel < 0 && hr.material.scatter_dispersive(&current, &hr, 0).is_some() {
                        for split_channel in 0..3usize {
                            if let Some((next_ray, _)) = hr.material.scatter_dispersive(&current, &hr, split_channel) {
                                next_active.push((next_ray, split_channel as i32, points.clone(), sources.clone(), -1));
                            }
                        }
                        continue;
                    }

                    // Mirrors Camera::ray_colour's specular/importance-sampled
                    // branch so a clicked pixel's traced path matches what
                    // the strategy actually does to the progressive render
                    // — but this loop stays its own reimplementation (not a
                    // call to ray_colour) since it also needs the
                    // dispersion-splitting and bundle bookkeeping above.
                    let use_pdf_sampling = channel < 0
                        && self.sampling_strategy != SamplingStrategy::Naive
                        && !hr.material.is_specular();
                    let scattered = if channel >= 0 {
                        hr.material.scatter_dispersive(&current, &hr, channel as usize).map(|(next_ray, _)| (next_ray, -1i32))
                    } else if use_pdf_sampling {
                        hr.material.scatter(&current, &hr, &mut rng).map(|(_, _attenuation)| {
                            let use_light = self.sampling_strategy == SamplingStrategy::Mixture
                                && !self.lights.is_empty()
                                && rng.random::<f64>() < 0.5;
                            let (direction, source) = if use_light {
                                (HittablePdf::new(self.lights.clone(), hr.point).generate(&mut rng), 1i32)
                            } else {
                                (CosinePdf::new(hr.normal).generate(&mut rng), 0i32)
                            };
                            (Ray::new_with_time(hr.point, direction, current.time), source)
                        })
                    } else {
                        hr.material.scatter(&current, &hr, &mut rng).map(|(next_ray, _)| (next_ray, -1i32))
                    };
                    match scattered {
                        None => paths.push(TracePath { points, sources, channel, termination: 1 }),
                        Some((next_ray, source)) => next_active.push((next_ray, channel, points, sources, source)),
                    }
                }
                active = next_active;
            }
            // Ran out of max_depth while still bouncing — same visual bucket as absorbed.
            for (_, channel, points, sources, _) in active {
                paths.push(TracePath { points, sources, channel, termination: 1 });
            }
        }

        let mut out = Vec::new();
        out.push(paths.len() as f64);
        for path in &paths {
            let mut coords = vec![i as f64, j as f64];
            let mut sources = vec![-1.0f64];
            for (idx, p) in path.points.iter().enumerate() {
                match self.camera.project_point(*p) {
                    Some((x, y)) => {
                        coords.push(x);
                        coords.push(y);
                        sources.push(*path.sources.get(idx).unwrap_or(&-1) as f64);
                    }
                    None => break,
                }
            }
            out.push(path.termination as f64);
            out.push(path.channel as f64);
            out.push((coords.len() / 2) as f64);
            out.extend(coords);
            out.extend(sources);
        }
        out
    }

    /// Fires the primary ray through pixel (i, j), finds the first
    /// non-emissive hit, and draws `n` sample directions from it per `mode`
    /// (0 = uniform hemisphere, 1 = cosine-weighted, 2 = light-only NEE —
    /// falls back to cosine if the scene has no lights), each projected a
    /// short fixed distance forward into screen space. Deliberately separate
    /// from `trace_pixel`: this is a single-bounce direction-*distribution*
    /// visualization ("sunburst" — why cosine sampling clusters near the
    /// normal while uniform-hemisphere doesn't), not a traced path, so it
    /// carries none of that function's recursion/termination/dispersion
    /// bookkeeping. Returns `[]` if the click misses everything or its hit
    /// point can't be projected; otherwise `[originX, originY, count, x0,
    /// y0, ..., x(count-1), y(count-1)]` — `count` may be less than `n` since
    /// a sample direction pointing back past the camera can't be projected.
    pub fn sample_directions(&self, i: u32, j: u32, mode: u32, n: u32) -> Vec<f64> {
        const SAMPLE_LENGTH: f64 = 0.6;
        let mut rng = rand::rng();
        let ray = self.camera.get_ray(i as usize, j as usize, &mut rng);
        let Some(hr) = self.world.hit(&ray, Interval::new(0.001, f64::INFINITY), &mut rng) else {
            return Vec::new();
        };
        let Some((oi, oj)) = self.camera.project_point(hr.point) else {
            return Vec::new();
        };

        let mut tips = Vec::new();
        for _ in 0..n {
            let direction = match mode {
                0 => UniformHemispherePdf::new(hr.normal).generate(&mut rng),
                2 if !self.lights.is_empty() => HittablePdf::new(self.lights.clone(), hr.point).generate(&mut rng),
                _ => CosinePdf::new(hr.normal).generate(&mut rng),
            };
            let tip = hr.point + SAMPLE_LENGTH * direction.normalize();
            if let Some((x, y)) = self.camera.project_point(tip) {
                tips.push(x);
                tips.push(y);
            }
        }

        let mut out = vec![oi, oj, (tips.len() / 2) as f64];
        out.extend(tips);
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

    fn rebuild_camera(&mut self) {
        self.camera = build_camera(
            self.width, self.height, self.vfov, self.lookat, self.theta, self.phi, self.orbit_radius,
            self.defocus_angle, self.background,
        );
    }

    /// Swaps in a whole new world + camera framing (a preset or a parsed
    /// custom scene) and resets the progressive render, mirroring
    /// `set_sphere_material`'s reset pattern.
    fn apply_setup(&mut self, setup: SceneSetup) {
        self.spheres = setup.spheres;
        self.world = setup.world;
        self.lights = setup.lights;
        self.lookat = setup.lookat;
        self.theta = setup.theta;
        self.phi = setup.phi;
        self.orbit_radius = setup.orbit_radius;
        self.vfov = setup.vfov;
        self.defocus_angle = setup.defocus_angle;
        self.background = setup.background;
        self.max_depth = setup.max_depth.max(1);
        self.rebuild_camera();
        self.reset_accumulation();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `wasm_bindgen`-annotated methods still compile as plain Rust under the
    // native test target, so these run via a normal `cargo test -p
    // raytracer-wasm` — no wasm32 involved. This is the correctness check
    // Phase C's plan calls for on the mixture/NEE path: the native CLI
    // (`raytracer/src/camera.rs::render_parallel`) is hardcoded to
    // `SamplingStrategy::Naive` and never exercises `lights`/`Cosine`/
    // `Mixture` at all, so a diff against it wouldn't catch a bug here.
    fn total_radiance(scene: &mut Scene, strategy: SamplingStrategy, samples: u32) -> f64 {
        scene.sampling_strategy = strategy;
        scene.render_pass(samples);
        let total: f64 = scene.accum.iter().map(|c| c.x + c.y + c.z).sum();
        scene.reset_accumulation();
        total
    }

    #[test]
    fn mixture_sampling_matches_naive_total_radiance_on_the_cornell_box() {
        let mut scene = Scene::new(30, 20);
        scene.load_preset(1); // Cornell box: the only preset with a populated `lights` list.
        assert!(!scene.lights.is_empty(), "cornell box preset should have a light");

        // Both strategies are unbiased estimators of the same rendering
        // equation, so their expected total radiance should agree even
        // though mixture sampling has much lower variance per sample. A wide
        // tolerance keeps this from flaking on the RNG while still catching
        // a real bug (e.g. a missing normalization or a flipped pdf ratio),
        // which tends to blow totals up or crush them towards zero rather
        // than shift them by a few tens of percent.
        let naive_total = total_radiance(&mut scene, SamplingStrategy::Naive, 64);
        let mixture_total = total_radiance(&mut scene, SamplingStrategy::Mixture, 64);

        assert!(naive_total.is_finite() && mixture_total.is_finite());
        assert!(naive_total > 0.0 && mixture_total > 0.0);

        let ratio = mixture_total / naive_total;
        assert!((0.4..2.5).contains(&ratio), "naive={naive_total} mixture={mixture_total} ratio={ratio}");
    }

    #[test]
    fn cosine_sampling_matches_naive_total_radiance_on_a_scene_with_no_lights() {
        let mut scene = Scene::new(30, 20);
        // Default classic scene: no explicit lights, so cosine-importance
        // sampling should reduce to the same direction distribution as the
        // book-1 approximation naive sampling already uses.
        assert!(scene.lights.is_empty());

        let naive_total = total_radiance(&mut scene, SamplingStrategy::Naive, 64);
        let cosine_total = total_radiance(&mut scene, SamplingStrategy::Cosine, 64);

        assert!(naive_total.is_finite() && cosine_total.is_finite());
        let ratio = cosine_total / naive_total;
        assert!((0.6..1.6).contains(&ratio), "naive={naive_total} cosine={cosine_total} ratio={ratio}");
    }

    // Parses trace_pixel's `[numPaths, termination, channel, len, x0,y0,...,
    // s0,s1,...]` wire format, mirroring App.tsx's parseTracedPaths — kept in
    // sync with it by hand, not shared code, since one's Rust and the other
    // TypeScript.
    fn parse_traced_paths(raw: &[f64]) -> Vec<(u8, i32, Vec<(f64, f64)>, Vec<i32>)> {
        if raw.is_empty() {
            return Vec::new();
        }
        let num_paths = raw[0] as usize;
        let mut paths = Vec::new();
        let mut offset = 1;
        for _ in 0..num_paths {
            let termination = raw[offset] as u8;
            let channel = raw[offset + 1] as i32;
            let len = raw[offset + 2] as usize;
            offset += 3;
            let coords: Vec<(f64, f64)> = raw[offset..offset + len * 2]
                .chunks(2)
                .map(|c| (c[0], c[1]))
                .collect();
            offset += len * 2;
            let sources: Vec<i32> = raw[offset..offset + len].iter().map(|s| *s as i32).collect();
            offset += len;
            paths.push((termination, channel, coords, sources));
        }
        paths
    }

    #[test]
    fn trace_pixel_tags_every_vertex_as_not_pdf_sampled_under_the_naive_strategy() {
        let mut scene = Scene::new(30, 20);
        scene.sampling_strategy = SamplingStrategy::Naive;
        let raw = scene.trace_pixel(15, 10, 8);
        let paths = parse_traced_paths(&raw);
        assert!(!paths.is_empty());
        for (_, _, coords, sources) in &paths {
            assert_eq!(coords.len(), sources.len());
            assert!(sources.iter().all(|s| *s == -1), "expected all -1, got {sources:?}");
        }
    }

    #[test]
    fn trace_pixel_only_ever_emits_known_pdf_source_tags_on_the_cornell_box_under_mixture() {
        let mut scene = Scene::new(30, 20);
        scene.load_preset(1); // Cornell box
        scene.sampling_strategy = SamplingStrategy::Mixture;
        // Click a handful of pixels — the exact mix of cosine/light/neutral
        // tags depends on where each ray happens to land, so this only
        // pins down the wire format's shape/legality, not the distribution.
        for (px, py) in [(15, 10), (10, 12), (20, 8), (12, 15), (18, 5)] {
            let raw = scene.trace_pixel(px, py, 8);
            let paths = parse_traced_paths(&raw);
            for (_, _, coords, sources) in &paths {
                assert_eq!(coords.len(), sources.len());
                assert!(sources.iter().all(|s| (-1..=1).contains(s)), "unexpected tag in {sources:?}");
            }
        }
    }

    #[test]
    fn render_snapshot_with_strategy_leaves_the_live_strategy_and_accumulator_untouched() {
        let mut scene = Scene::new(10, 8);
        scene.sampling_strategy = SamplingStrategy::Naive;
        scene.render_pass(2);
        let accum_before = scene.accum.clone();

        for mode in 0..3u32 {
            let out = scene.render_snapshot_with_strategy(mode, 1);
            assert_eq!(out.len(), 10 * 8 * 4);
        }

        assert_eq!(scene.sampling_strategy, SamplingStrategy::Naive);
        assert_eq!(scene.accum, accum_before);
    }

    #[test]
    fn render_snapshot_with_strategy_produces_a_visibly_lit_image_not_all_black() {
        let mut scene = Scene::new(20, 15);
        scene.load_preset(1); // Cornell box: lit interior, easy to notice an all-black regression.
        for mode in 0..3u32 {
            let out = scene.render_snapshot_with_strategy(mode, 4);
            let brightest = out.iter().copied().max().unwrap_or(0);
            assert!(brightest > 10, "mode {mode} rendered an all-black (or near-black) snapshot: max byte = {brightest}");
        }
    }

    #[test]
    fn sample_directions_returns_the_requested_number_of_tips_or_fewer() {
        let scene = Scene::new(30, 20);
        for mode in 0..3u32 {
            let raw = scene.sample_directions(15, 10, mode, 12);
            if raw.is_empty() {
                continue; // that pixel's primary ray missed everything
            }
            let count = raw[2] as usize;
            assert!(count <= 12);
            assert_eq!(raw.len(), 3 + count * 2);
        }
    }
}
