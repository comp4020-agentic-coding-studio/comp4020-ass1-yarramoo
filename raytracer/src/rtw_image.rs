use std::path::Path;

use image::{Pixel, Rgb, RgbImage};

use crate::colour::gamma_to_linear;

#[derive(Debug)]
pub struct RTWImage {
    pub width: usize,
    pub height: usize, 
    pub image: RgbImage,
}

impl RTWImage {
    pub fn new(path: &Path) -> Result<Self, image::ImageError> {
        let mut image = image::open(path)?.to_rgb8();
        for (_, _, pixel) in image.enumerate_pixels_mut() {
            pixel.apply(gamma_to_linear);
        }
        // Convert to linear space
        Ok(Self { width: image.width() as usize, height: image.height() as usize, image })
    }

    fn clamp(x: usize, low: usize, high: usize) -> usize {
        if x < low { low }
        else if x < high { x } 
        else { high - 1 }
    }

    pub fn pixel_data(&self, x: usize, y: usize) -> &Rgb<u8> {
        let x = Self::clamp(x, 0, self.width) as u32;
        let y = Self::clamp(y, 0, self.height) as u32;
        self.image.get_pixel(x, y)
    }
}