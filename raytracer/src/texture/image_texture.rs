use std::path::Path;

use crate::{colour::Colour, interval::Interval, rtw_image::RTWImage, texture::{Texture, TextureCoords}, vector::V3};


#[derive(Debug)]
pub struct ImageTexture {
    image: RTWImage,
}

impl ImageTexture {
    pub fn new(path: &Path) -> Result<Self, image::ImageError> {
        let image = RTWImage::new(path)?;
        Ok(Self { image })
    }
}

impl Texture for ImageTexture {
    fn value(&self, coords: TextureCoords, _p: V3) -> Colour {
        let u = Interval::new(0., 1.).clamp(coords.x);
        let v = 1. - Interval::new(0., 1.).clamp(coords.y);

        let i = (u * self.image.width as f64) as usize;
        let j = (v * self.image.height as f64) as usize;
        let pixel = self.image.pixel_data(i, j);

        let colour_scale = 1. / 255.;
        Colour::new(
            pixel[0] as f64 * colour_scale,
            pixel[1] as f64 * colour_scale,
            pixel[2] as f64 * colour_scale,
        )
    }
}