use image::{GenericImageView, Pixel, Rgb, Rgba};

mod median_cut;
mod neu;

pub use median_cut::MedianCut;
pub use neu::Neu;
use std::ops::Range;

/// Color with population
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct Color {
    /// Color
    pub color: Rgb<u8>,
    /// Population
    pub population: usize,
}

/// Errors when using a quantizer
#[derive(Debug)]
pub enum Error {
    /// Quality was out of bounds
    QualityOutOfBounds(u32, Range<usize>),
    /// Color was out of bounds
    ColorCountOutOfBounds(usize, Range<usize>),
}

/// Quantizer trait
pub trait Quantizer {
    /// Quantizes the input image into the given color count using all pixels for which filter returns true
    fn quantize<I, P, F>(
        &self,
        image: &I,
        colors: usize,
        quality: u32,
        filter: F,
    ) -> Result<Vec<Color>, Error>
    where
        P: Pixel<Subpixel = u8> + 'static,
        I: GenericImageView<Pixel = P>,
        F: FnMut(&Rgba<u8>) -> bool;
}
