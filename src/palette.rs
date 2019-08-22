use std::fmt;
use std::collections::HashMap;

use itertools::Itertools;
use image::{FilterType, DynamicImage, GenericImageView, Pixel, Rgb};
use crate::quantizer::quantize_pixels;
use termion::color;

/// Color swatch generated from an image's palette.
pub struct Swatch {
    /// RGB color
    pub rgb: Rgb<u8>,
    /// Number of pixels that are approximated by this color
    pub population: usize,
}

/// Palette of colors.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct Palette {
    /// Palette of Colors represented in RGB
    pub palette: Vec<Rgb<u8>>,
    /// A map of indices in the palette to a count of pixels in approximately that color in the
    /// original image.
    pub pixel_counts: HashMap<usize, usize>,
}

impl Palette {
    /// Create a new palett from an image
    ///
    /// Color count and quality are given straight to [color_quant], values should be between
    /// 8...512 and 1...30 respectively. (By the way: 10 is a good default quality.)
    ///
    /// [color_quant]: https://github.com/PistonDevelopers/color_quant
    pub fn new(image: &DynamicImage, color_count: usize, quality: usize) -> Palette
    {
        // resize image to reduce computational complexity
        let image = image.resize((image.width() as f32 / quality as f32) as u32, (image.height() as f32 / quality as f32) as u32,
            FilterType::Triangle);

        //println!("shrunk image to {}x{}", image.width(), image.height());

        let pixels: Vec<Rgb<u8>> = image.pixels()
                                         .map(|(_, _, pixel)| pixel.to_rgb())
                                         .collect();

        let mut pixels_cleaned = pixels.clone();
        pixels_cleaned.retain(|i| !is_boring_pixel(&i));

        //println!("hashset: {}, original: {}, cleaned: {}", pixel_hashset.len(), pixels.len(), pixels_cleaned.len());

        let quant = quantize_pixels(color_count, &mut pixels_cleaned);


        let pixel_counts = quant.iter().enumerate().fold(HashMap::new(), |mut m, (i, p)| { m.insert(i, p.population); m });

        let palette_pixels = quant.iter().fold(Vec::<Rgb<u8>>::new(), |mut v, p| { v.push(p.rgb); v});

        //for i in quant{
        //    println!("{:?} {}\t{}███{}", i.rgb, i.population, color::Fg(color::Rgb(i.rgb[0], i.rgb[1], i.rgb[2])),
        //            color::Fg(color::Reset));
        //}

        Palette {
            palette: palette_pixels,
            pixel_counts: pixel_counts,
        }
    }

    fn frequency_of(&self, color: &Rgb<u8>) -> usize {
        let index = self.palette.iter().position(|x| x.channels() == color.channels());
        if let Some(index) = index {
            *self.pixel_counts.get(&index).unwrap_or(&0)
        } else {
            0
        }
    }

    /// Change ordering of colors in palette to be of frequency using the pixel count.
    pub fn sort_by_frequency(&self) -> Self {
        let mut colors = self.palette.clone();
        colors.sort_by(|a, b| self.frequency_of(&a).cmp(&self.frequency_of(&b)));

        Palette {
            palette: colors,
            pixel_counts: self.pixel_counts.clone(),
        }
    }
}

fn is_boring_pixel(pixel: &Rgb<u8>) -> bool {
    let (r, g, b) = (pixel[0], pixel[1], pixel[2]);

    // If pixel is mostly opaque and not white
    //const MIN_ALPHA: u8 = 125;
    const MAX_COLOR: u8 = 250;

    let interesting = !(r > MAX_COLOR && g > MAX_COLOR && b > MAX_COLOR);

    !interesting
}

impl fmt::Display for Palette {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let color_list = self.palette
                             .iter()
                             .map(|rgb| format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]))
                             .join(", ");

        write!(f, "Color Palette {{ {} }}", color_list)
    }
}
