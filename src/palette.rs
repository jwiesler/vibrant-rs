use std::fmt;

use image::imageops::FilterType;
use image::{DynamicImage, GenericImage, GenericImageView, Pixel, Rgb, Rgba};
use itertools::Itertools;

use crate::quantize;

/// Palette of colors.
#[derive(Debug, Default)]
pub struct Palette {
    /// Palette of Colors
    pub palette: Vec<Color>,
}

/// Color with population
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct Color {
    /// Color
    pub color: Rgb<u8>,
    /// Population
    pub population: usize,
}

impl Palette {
    /// Create a new palette from an image
    /// Downscales first by factor `1/quality`
    pub fn new(image: &DynamicImage, color_count: usize, quality: u32) -> Palette {
        let factor = 1.0 / quality as f64;
        let image = image.resize(
            (image.width() as f64 * factor).round() as u32,
            (image.height() as f64 * factor).round() as u32,
            FilterType::Gaussian,
        );
        Self::from_image(&image, color_count)
    }

    /// Create a new palette from an image
    pub fn from_image<P, G>(image: &G, color_count: usize) -> Palette
    where
        P: Sized + Pixel<Subpixel = u8>,
        G: Sized + GenericImage<Pixel = P>,
    {
        let pixels: Vec<Rgba<u8>> = image
            .pixels()
            .map(|(_, _, pixel)| pixel.to_rgba())
            .collect();
        let palette = quantize(&pixels, color_count, is_interesting_pixel);
        Palette { palette }
    }

    fn frequency_of(&self, color: &Rgb<u8>) -> usize {
        self.palette
            .iter()
            .find(|x| x.color.channels() == color.channels())
            .map(|c| c.population)
            .unwrap_or(0)
    }

    /// Change ordering of colors in palette to be of frequency using the pixel count.
    pub fn sort_by_frequency(&self) -> Self {
        let mut palette = self.palette.clone();
        palette.sort_by_key(|value| self.frequency_of(&value.color));
        Self { palette }
    }
}

fn is_interesting_pixel(pixel: &Rgba<u8>) -> bool {
    let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);

    // If pixel is mostly opaque and not white
    const MIN_ALPHA: u8 = 125;
    const MAX_COLOR: u8 = 250;

    let interesting = (a >= MIN_ALPHA) && !(r > MAX_COLOR && g > MAX_COLOR && b > MAX_COLOR);

    interesting
}

impl fmt::Display for Palette {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let color_list = self
            .palette
            .iter()
            .map(|c| c.color)
            .map(|rgb| format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]))
            .join(", ");

        write!(f, "Color Palette {{ {} }}", color_list)
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rgb = self.color.channels();
        write!(f, "#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])?;

        write!(f, ", {} pixels", self.population)
    }
}
