use std::fmt;

use color_quant::NeuQuant;
use image::{GenericImage, Pixel, Rgb, Rgba};
use itertools::Itertools;

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
    ///
    /// Color count and quality are given straight to [color_quant], values should be between
    /// 8...512 and 1...30 respectively. (By the way: 10 is a good default quality.)
    ///
    /// [color_quant]: https://github.com/PistonDevelopers/color_quant
    pub fn new<P, G>(image: &G, color_count: usize, quality: i32) -> Palette
    where
        P: Sized + Pixel<Subpixel = u8>,
        G: Sized + GenericImage<Pixel = P>,
    {
        let pixels: Vec<Rgba<u8>> = image
            .pixels()
            .map(|(_, _, pixel)| pixel.to_rgba())
            .collect();

        let mut flat_pixels: Vec<u8> = Vec::with_capacity(pixels.len());
        for rgba in &pixels {
            if is_boring_pixel(&rgba) {
                continue;
            }

            for subpixel in rgba.channels() {
                flat_pixels.push(*subpixel);
            }
        }

        let quantize = NeuQuant::new(quality, color_count, &flat_pixels);

        let pixel_counts = pixels
            .iter()
            .map(|rgba| quantize.index_of(&rgba.channels()))
            .counts();

        let palette = quantize
            .color_map_rgb()
            .chunks_exact(3)
            .enumerate()
            .flat_map(|(i, rgb)| pixel_counts.get(&i).map(|&count| (count, rgb)))
            .map(|(count, rgb)| Color {
                color: *Rgb::from_slice(rgb),
                population: count,
            })
            .unique_by(|c| c.color)
            .collect();

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

fn is_boring_pixel(pixel: &Rgba<u8>) -> bool {
    let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);

    // If pixel is mostly opaque and not white
    const MIN_ALPHA: u8 = 125;
    const MAX_COLOR: u8 = 250;

    let interesting = (a >= MIN_ALPHA) && !(r > MAX_COLOR && g > MAX_COLOR && b > MAX_COLOR);

    !interesting
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
