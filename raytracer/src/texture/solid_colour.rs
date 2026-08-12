use crate::{colour::Colour, texture::{Texture, TextureCoords}, vector::V3};


#[derive(Debug)]
pub struct SolidColour {
    albedo: Colour,
}

impl SolidColour {
    pub fn new(albedo: Colour) -> Self {
        Self { albedo }
    }
}

impl Texture for SolidColour {
    fn value(&self, _coords: TextureCoords, _p: V3) -> Colour {
        self.albedo 
    }
}