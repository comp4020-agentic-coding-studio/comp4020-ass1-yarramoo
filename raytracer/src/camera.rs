use core::f64;
use std::{io::{self, Write}, sync::Arc};

use rand::{Rng, RngCore};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{colour::{Colour, write_colour}, hittable::Hittable, interval::Interval, ray::Ray, sample::{self, Sample}, vector::{V3, random_in_unit_disk}};

#[derive(Default)]
pub struct Render {
    pub pixels: Vec<Colour>,
    pub width: usize,
    pub height: usize,
}

impl Render {
    pub fn write_ppm(&self, writer: &mut impl Write) -> io::Result<()> {
        let header = format!("P3\n{} {}\n256\n", self.width, self.height);
        let _ = writer.write(header.as_bytes())?;
        for pixel in &self.pixels {
            write_colour(writer, pixel)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct CameraBuilder {
    image_width: Option<usize>,
    image_height: Option<usize>,
    aspect_ratio: Option<f64>,
    samples_per_pixel: Option<usize>,
    max_depth: Option<usize>,
    vfov: Option<f64>,
    lookfrom: Option<V3>,
    lookat: Option<V3>,
    vup: Option<V3>,
    focus_distance: Option<f64>,
    defocus_angle: Option<f64>,
    background: Option<Colour>,
}

#[derive(Debug)]
pub enum CameraBuilderError {
    InsufficientImageDimensions,
}

impl CameraBuilder {
    const DEFAULT_PIXEL_SAMPLES: usize = 10;
    const DEFAULT_MAX_DEPTH: usize = 10;
    const DEFAULT_VFOV: f64 = 70.;
    const DEFAULT_LOOKFROM: V3 = V3::new(0., 0., 0.);
    const DEFAULT_LOOKAT: V3 = V3::new(0., 0., -1.);
    const DEFAULT_VUP: V3 = V3::new(0., 1., 0.);
    const DEFAULT_FOCUS_DISTANCE: f64 = 10.;
    const DEFAULT_DEFOCUS_ANGLE: f64 = 0.;
    const DEFAULT_BACKGROUND: Colour = Colour::new(0.70, 0.80, 1.00);

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.image_width = Some(width);
        self
    }

    pub fn with_height(mut self, height: usize) -> Self {
        self.image_height = Some(height);
        self
    }

    pub fn with_aspect_ratio(mut self, ratio: f64) -> Self {
        self.aspect_ratio = Some(ratio);
        self
    }

    pub fn with_samples_per_pixel(mut self, sample_count: usize) -> Self {
        self.samples_per_pixel = Some(sample_count);
        self
    }

    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    pub fn with_vfov(mut self, vfov: f64) -> Self {
        self.vfov = Some(vfov);
        self
    }

    pub fn with_camera_pos(mut self, lookfrom: V3, lookat: V3, vup: V3) -> Self {
        self.lookfrom = Some(lookfrom);
        self.lookat = Some(lookat);
        self.vup = Some(vup);
        self
    }

    pub fn with_defocus(mut self, distance: f64, angle: f64) -> Self {
        self.focus_distance = Some(distance);
        self.defocus_angle = Some(angle);
        self
    }

    pub fn with_background(mut self, background: Colour) -> Self {
        self.background = Some(background);
        self
    }

    pub fn build(self) -> Result<Camera, CameraBuilderError> {
        // Get image dimensions if we have combination of width, height, aspect ratio
        let (image_width, aspect_ratio) = 
            if let (Some(height), Some(width)) = (self.image_height, self.image_width) 
            {
                (width, width as f64 / height as f64)
            }
            else if let (Some(aspect_ratio), Some(width)) = (self.aspect_ratio, self.image_width)
            {
                (width, aspect_ratio)
            }
            else if let (Some(aspect_ratio), Some(height)) = (self.aspect_ratio, self.image_height)
            {
                ((aspect_ratio * height as f64) as usize, aspect_ratio)
            }
            else {
                return Err(CameraBuilderError::InsufficientImageDimensions);
            };

        let samples_per_pixel = self.samples_per_pixel.unwrap_or(Self::DEFAULT_PIXEL_SAMPLES);
        let max_depth = self.max_depth.unwrap_or(Self::DEFAULT_MAX_DEPTH);
        let vfov = self.vfov.unwrap_or(Self::DEFAULT_VFOV);
        let lookfrom = self.lookfrom.unwrap_or(Self::DEFAULT_LOOKFROM);
        let lookat = self.lookat.unwrap_or(Self::DEFAULT_LOOKAT);
        let vup = self.vup.unwrap_or(Self::DEFAULT_VUP);
        let focus_distance = self.focus_distance.unwrap_or(Self::DEFAULT_FOCUS_DISTANCE);
        let defocus_angle = self.defocus_angle.unwrap_or(Self::DEFAULT_DEFOCUS_ANGLE);
        let background = self.background.unwrap_or(Self::DEFAULT_BACKGROUND);

        Ok(Camera::new(
            aspect_ratio,
            image_width,
            samples_per_pixel,
            max_depth,
            vfov,
            lookfrom,
            lookat,
            vup,
            focus_distance,
            defocus_angle,
            background,
        ))
    }
}

pub struct Camera {
    // aspect_ratio: f64,
    image_width: usize,
    image_height: usize,
    centre: V3,
    pixel00_loc: V3,
    pixel_delta_u: V3,
    pixel_delta_v: V3,
    samples_per_pixel: usize,
    pixel_samples_scale: f64,
    max_depth: usize,
    defocus_angle: f64,
    defocus_disk_u: V3,
    defocus_disk_v: V3,
    // vfov: f64,
    // basis_vectors: BasisVectors,
    background: Colour,
}

struct BasisVectors {
    u: V3, v: V3, w: V3,
}

impl BasisVectors {
    fn from_camera_pos(lookfrom: V3, lookat: V3, vup: V3) -> Self {
        let w = (lookfrom - lookat).normalize();
        let u = vup.cross(&w).normalize();
        let v = w.cross(&u);
        Self { u, v, w }
    }
}


impl Camera {
    fn new(
        aspect_ratio: f64, 
        image_width: usize, 
        samples_per_pixel: usize, 
        max_depth: usize, 
        vfov: f64,
        lookfrom: V3,
        lookat: V3,
        vup: V3,
        focus_distance: f64,
        defocus_angle: f64,
        background: Colour,
    ) -> Self 
    {
        // Image height (at least 1)
        let image_height = {
            let tmp = (image_width as f64 / aspect_ratio) as usize;
            if tmp < 1 { 1 } else { tmp }
        };

        // Camera
        let theta = vfov.to_radians();
        let h = (theta / 2.).tan();
        let viewport_height = 2. * h * focus_distance;
        let viewport_width = viewport_height * (image_width as f64 / image_height as f64);

        // Basis
        let basis = BasisVectors::from_camera_pos(lookfrom, lookat, vup);

        // Vectors across and down viewport
        let viewport_u = viewport_width * basis.u;
        let viewport_v = viewport_height * -basis.v;

        // Horizontal and vertical delta between pixels
        let pixel_delta_u = viewport_u / image_width as f64;
        let pixel_delta_v = viewport_v / image_height as f64;

        // Find upper left viewport corner and pixel
        let viewport_upper_left = lookfrom 
            - (focus_distance * basis.w) 
            - (viewport_u + viewport_v) / 2.;
        let pixel00_loc = 
            viewport_upper_left + (pixel_delta_u + pixel_delta_v) / 2.;

        // Find camera defocus disk basis vectors
        let defocus_radius = focus_distance * (defocus_angle / 2.).to_radians().tan();
        let defocus_disk_u = basis.u * defocus_radius;
        let defocus_disk_v = basis.v * defocus_radius;

        Self {
            // aspect_ratio,
            image_width,
            image_height,
            centre: lookfrom,
            pixel00_loc,
            pixel_delta_u,
            pixel_delta_v,
            samples_per_pixel,
            pixel_samples_scale: 1. / samples_per_pixel as f64,
            max_depth,
            defocus_angle,
            defocus_disk_u,
            defocus_disk_v,
            // vfov,
            // basis_vectors: basis,
            background
        }
    }

    pub fn render_parallel(&self, world: &impl Hittable) -> Render {
        let w = Arc::new(world);

        let pixels: Vec<Colour> = (0..self.image_height)
            .into_par_iter()
            .flat_map(|j| {
                let world = Arc::clone(&w);
                let mut rng = rand::rng();
                
                (0..self.image_width).map(move |i| {
                    let mut colour = Colour::default();
                    for _ in 0..self.samples_per_pixel {
                        let ray = self.get_ray(i, j, &mut rng);
                        colour += self.ray_colour(&ray, *world, self.max_depth, &mut rng);
                    }
                    self.pixel_samples_scale * colour
                }).collect::<Vec<_>>()
            })
            .collect();


        Render {
            pixels, 
            width: self.image_width,
            height: self.image_height,
        }
    }

    pub fn get_ray(&self, i: usize, j: usize, rng: &mut impl Rng) -> Ray {
        let offset = sample::Square::sample(rng);
        let pixel_sample = self.pixel00_loc
            + (i as f64 + offset.x) * self.pixel_delta_u
            + (j as f64 + offset.y) * self.pixel_delta_v;

        let ray_origin = if self.defocus_angle != 0. { self.defocus_disk_sample(rng) } else { self.centre };
        let ray_direction = pixel_sample - ray_origin;
        let ray_time = rng.random();

        Ray::new_with_time(ray_origin, ray_direction, ray_time)
    }

    /// Projects a 3D world point onto this camera's pixel grid as fractional
    /// (i, j). Returns None if the point is behind the camera or the
    /// sightline runs parallel to the image plane — both occur for real
    /// bounce paths (e.g. a ray reflecting back past the camera), so callers
    /// must handle it by truncating rather than drawing garbage.
    pub fn project_point(&self, p: V3) -> Option<(f64, f64)> {
        let direction = p - self.centre;
        if direction.norm_squared() < 1e-12 { return None; }
        // pixel_delta_u ⊥ pixel_delta_v by construction, so their cross
        // product is a valid (unnormalized) image-plane normal.
        let normal = self.pixel_delta_u.cross(&self.pixel_delta_v);
        let denom = direction.dot(&normal);
        if denom.abs() < 1e-9 { return None; }
        let t = (self.pixel00_loc - self.centre).dot(&normal) / denom;
        if t <= 0.0 { return None; }
        let hit = self.centre + t * direction;
        let rel = hit - self.pixel00_loc;
        let i = rel.dot(&self.pixel_delta_u) / self.pixel_delta_u.dot(&self.pixel_delta_u);
        let j = rel.dot(&self.pixel_delta_v) / self.pixel_delta_v.dot(&self.pixel_delta_v);
        Some((i, j))
    }

    pub fn ray_colour(&self, ray: &Ray, world: &impl Hittable, depth: usize, rng: &mut dyn RngCore) -> Colour {
        if depth == 0 { return V3::default(); }

        let hr = world.hit(ray, Interval::new(0.001, f64::INFINITY), rng);
        if hr.is_none() {
            return self.background;
        }
        let hr = hr.unwrap();
        let emission = hr.material.emitted(hr.texture_coords, hr.point);
        let scatter = hr.material.scatter(ray, &hr, rng);
        if scatter.is_none() {
            return emission;
        }
        let (scatter, attenuation) = scatter.unwrap();
        let scatter_colour = attenuation.component_mul(&self.ray_colour(&scatter, world, depth-1, rng));
        emission + scatter_colour
    }

    fn defocus_disk_sample(&self, rng: &mut impl Rng) -> V3 {
        let p = random_in_unit_disk(rng);
        self.centre + (p.x * self.defocus_disk_u) + (p.y * self.defocus_disk_v)
    }


}
