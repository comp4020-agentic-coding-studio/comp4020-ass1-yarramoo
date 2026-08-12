use crate::{colour::Colour, texture::{Texture, TextureCoords}, vector::V3};


#[derive(Debug)]
pub struct BarycentricInterpolation {
    colour_a: Colour,
    colour_b: Colour,
    colour_c: Colour
}

impl BarycentricInterpolation {
    pub fn new(colour_a: Colour, colour_b: Colour, colour_c: Colour) -> Self {
        Self { colour_a, colour_b, colour_c }
    }
}

impl Texture for BarycentricInterpolation {
    fn value(&self, coords: TextureCoords, _p: V3) -> Colour {
        (1. - coords.x - coords.y) * self.colour_a
        + coords.x * self.colour_b
        + coords.y * self.colour_c
    }
}