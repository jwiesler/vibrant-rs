use std::fmt;

use image::{GenericImageView, Pixel, Rgba};
use itertools::Itertools;

use crate::{Color, Error, Quantizer};

/// Palette of colors.
#[derive(Debug, Default)]
pub struct Palette {
    /// Palette of Colors
    pub palette: Vec<Color>,
}

impl Palette {
    /// Create a new palette from an image
    pub fn from_image<P, G, Q>(
        image: &G,
        color_count: usize,
        quality: u32,
        quantizer: &Q,
    ) -> Result<Palette, Error>
    where
        P: Pixel<Subpixel = u8> + 'static,
        G: GenericImageView<Pixel = P>,
        Q: Quantizer,
    {
        let palette = quantizer.quantize(image, color_count, quality, is_interesting_pixel)?;
        Ok(Self { palette })
    }

    /// Change ordering of colors in palette to be of frequency using the pixel count.
    pub fn into_sorted_by_frequency(mut self) -> Self {
        self.palette.sort_by_key(|value| value.population);
        self
    }
}

fn is_interesting_pixel(pixel: &Rgba<u8>) -> bool {
    let (r, g, b, a) = (pixel[0], pixel[1], pixel[2], pixel[3]);

    // If pixel is mostly opaque and not white
    const MIN_ALPHA: u8 = 125;
    const MAX_COLOR: u8 = 250;

    (a >= MIN_ALPHA) && !(r > MAX_COLOR && g > MAX_COLOR && b > MAX_COLOR)
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
