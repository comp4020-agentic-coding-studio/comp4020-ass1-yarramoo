use crate::{colour::Colour, perlin::Perlin, texture::{Texture, TextureCoords}, vector::V3};


#[derive(Debug)]
pub struct NoiseTexture {
    noise: Perlin,
    scale: f64,
}

impl NoiseTexture {
    pub fn new(scale: f64) -> Self {
        Self { noise: Perlin::new(), scale }
    }
}

impl Texture for NoiseTexture {
    fn value(&self, _coords: TextureCoords, p: V3) -> Colour {
        Colour::new(1.,1.,1.) * 0.5 * (1. + self.noise.noise(self.scale * p))
    }
}

#[derive(Default, Debug)]
pub struct TurbulenceTexture {
    noise: Perlin,
}

impl TurbulenceTexture {
    pub fn new() -> Self {
        Self { noise: Perlin::new() }
    }
}

impl Texture for TurbulenceTexture {
    fn value(&self, _coords: TextureCoords, p: V3) -> Colour {
        Colour::new(1., 1., 1.) * self.noise.turb(p, 7)
    }
}

#[derive(Debug)]
pub struct MarbleTexture {
    noise: Perlin,
    scale: f64
}

impl MarbleTexture {
    pub fn new(scale: f64) -> Self {
        Self { noise: Perlin::new(), scale }
    }
}

impl Texture for MarbleTexture {
    fn value(&self, _coords: TextureCoords, p: V3) -> Colour {
        Colour::new(0.5,0.5,0.5) * (1. + f64::sin(self.scale * p.z + 10. * self.noise.turb(p, 7)))
    }
}