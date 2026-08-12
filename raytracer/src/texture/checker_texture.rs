use crate::{colour::Colour, texture::{Texture, TextureCoords, solid_colour::SolidColour}, vector::V3};

#[derive(Debug)]
pub struct CheckerTexture {
    inv_scale: f64,
    even: Box<dyn Texture>,
    odd: Box<dyn Texture>,
}

impl CheckerTexture {
    pub fn new(scale: f64, even: Box<dyn Texture>, odd: Box<dyn Texture>) -> Self {
        Self { inv_scale: 1. / scale, even, odd}
    }

    pub fn from_solid(scale: f64, even: Colour, odd: Colour) -> Self {
        let even = SolidColour::new(even);
        let odd = SolidColour::new(odd);
        Self::new(scale, Box::new(even), Box::new(odd))
    }
}

impl Texture for CheckerTexture {
    fn value(&self, coords: TextureCoords, p: V3) -> Colour {
        let x = (self.inv_scale * p.x).floor() as isize;
        let y = (self.inv_scale * p.y).floor() as isize;
        let z = (self.inv_scale * p.z).floor() as isize;
        let is_even = (x + y + z) % 2 == 0;
        if is_even {
            self.even.value(coords, p)
        } else {
            self.odd.value(coords, p)
        }
    }
}