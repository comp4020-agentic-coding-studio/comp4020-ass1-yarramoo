pub mod solid_colour;
pub mod checker_texture;
pub mod image_texture;
pub mod noise_texture;
pub mod barycentric_interpolation;

use crate::{colour::Colour, vector::{V2, V3}};


pub type TextureCoords = V2;

pub trait Texture: Send + Sync + std::fmt::Debug {
    fn value(&self, coords: TextureCoords, p: V3) -> Colour;
}