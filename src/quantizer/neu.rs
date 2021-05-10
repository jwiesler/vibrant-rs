use crate::{Color, Error, Quantizer};
use color_quant::NeuQuant;
use image::{GenericImageView, Pixel, Rgb, Rgba};
use itertools::Itertools;

/// Neuronal network based quantizer
#[derive(Debug, Default)]
pub struct Neu;

impl Quantizer for Neu {
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
        F: FnMut(&Rgba<u8>) -> bool,
    {
        if !(1..31).contains(&quality) {
            return Err(Error::QualityOutOfBounds(quality, 1..31));
        }
        if !(64..266).contains(&colors) {
            return Err(Error::ColorCountOutOfBounds(colors, 64..266));
        }

        let pixels = image
            .pixels()
            .map(|(_, _, pixel)| pixel.to_rgba())
            .filter(filter);

        let mut flat_pixels: Vec<u8> =
            Vec::with_capacity(4 * image.height() as usize * image.width() as usize);
        for rgba in pixels {
            for subpixel in &rgba.0 {
                flat_pixels.push(*subpixel);
            }
        }

        let quantize = NeuQuant::new(quality as i32, colors, &flat_pixels);

        let pixel_counts = flat_pixels
            .chunks_exact(4)
            .map(|rgba| quantize.index_of(rgba))
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
        Ok(palette)
    }
}
