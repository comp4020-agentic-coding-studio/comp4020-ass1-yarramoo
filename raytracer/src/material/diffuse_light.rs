use std::sync::Arc;

use crate::{colour::Colour, material::Material, texture::{Texture, TextureCoords, solid_colour::SolidColour}, vector::V3};


#[derive(Debug)]
pub struct DiffuseLight {
    texture: Arc<dyn Texture>,
}

impl DiffuseLight {
    pub fn new(texture: &Arc<dyn Texture>) -> Self {
        Self { texture: Arc::clone(texture) }
    }

    pub fn from_colour(colour: Colour) -> Self {
        Self {
            texture: Arc::new(SolidColour::new(colour))
        }
    }
}

impl Material for DiffuseLight {
    fn emitted(&self, texture_coords: TextureCoords, p: V3) -> Colour { 
        self.texture.value(texture_coords, p)
    }
}