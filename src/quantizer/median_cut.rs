use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::Range;

use image::{
    imageops::{resize, FilterType},
    GenericImageView, Pixel, Rgba,
};

use crate::{Color, Error, Quantizer};

const BITS: usize = 5;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
struct Rgb<T> {
    r: T,
    g: T,
    b: T,
}

impl<T> Rgb<T> {
    fn map<O>(self, mut f: impl FnMut(T) -> O) -> Rgb<O> {
        Rgb {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
        }
    }

    fn as_mut(&mut self) -> Rgb<&mut T> {
        Rgb {
            r: &mut self.r,
            g: &mut self.g,
            b: &mut self.b,
        }
    }

    fn zip<O>(self, other: Rgb<O>) -> Rgb<(T, O)> {
        Rgb {
            r: (self.r, other.r),
            g: (self.g, other.g),
            b: (self.b, other.b),
        }
    }
}

impl Rgb<u8> {
    fn into_image_rgb(self) -> image::Rgb<u8> {
        image::Rgb {
            0: [self.r, self.g, self.b],
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct MinMax<T> {
    min: T,
    max: T,
}

impl<T: Ord + Copy> MinMax<T> {
    fn from_value(value: T) -> Self {
        Self {
            min: value,
            max: value,
        }
    }

    fn extend(&mut self, value: T) {
        if value < self.min {
            self.min = value;
        }
        if self.max < value {
            self.max = value;
        }
    }
}

impl MinMax<Quantized> {
    fn len(&self) -> usize {
        self.max.as_usize() - self.min.as_usize() + 1
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct Quantized(u8);

impl Quantized {
    fn from_color(color: u8) -> Self {
        Self(color >> (8 - BITS))
    }

    fn from_value_unchecked(value: usize) -> Self {
        debug_assert!(value < 1 << BITS);
        Self(value as u8)
    }

    fn to_color(self) -> u8 {
        self.0 << (8 - BITS)
    }

    fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl Rgb<Quantized> {
    fn as_color_index(&self) -> usize {
        let &Rgb { r, g, b } = self;
        (r.as_usize() << (2 * BITS)) | (g.as_usize() << BITS) | b.as_usize()
    }
}

struct Histogram {
    buckets: Vec<u32>,
}

impl Histogram {
    fn new() -> Self {
        Self {
            buckets: vec![0; 1 << (3 * BITS)],
        }
    }

    fn from_image<F: FnMut(&Rgba<u8>) -> bool>(
        image: impl IntoIterator<Item = Rgba<u8>>,
        f: F,
    ) -> (Self, Vec<Rgb<Quantized>>) {
        let mut histogram = Self::new();
        let iter = image.into_iter().filter(f).map(|color| {
            let [r, g, b, _] = color.0;
            Rgb { r, g, b }.map(Quantized::from_color)
        });

        for color in iter {
            histogram.insert(&color);
        }

        let unique_colors = histogram.counts().filter(|v| v != &0).count();
        let mut unique_colors = Vec::with_capacity(unique_colors);
        unique_colors.extend(
            histogram
                .buckets()
                .filter(|(_, count)| count != &0)
                .map(|(color, _)| color),
        );
        (histogram, unique_colors)
    }

    fn counts(&self) -> impl Iterator<Item = u32> + '_ {
        self.buckets.iter().copied()
    }

    fn buckets(&self) -> impl Iterator<Item = (Rgb<Quantized>, u32)> + '_ {
        self.buckets.iter().enumerate().map(|(color, &count)| {
            const MASK: usize = 0xFF >> (8 - BITS);
            (
                Rgb {
                    r: Quantized::from_value_unchecked(color >> 2 * BITS),
                    g: Quantized::from_value_unchecked((color >> BITS) & MASK),
                    b: Quantized::from_value_unchecked(color & MASK),
                },
                count,
            )
        })
    }

    fn insert(&mut self, color: &Rgb<Quantized>) {
        let index = color.as_color_index();
        self.buckets[index] += 1;
    }

    fn count_of(&self, color: &Rgb<Quantized>) -> u32 {
        let index = color.as_color_index();
        self.buckets[index]
    }

    fn colors<'a>(
        &'a self,
        colors: &'a [Rgb<Quantized>],
    ) -> impl Iterator<Item = (Rgb<Quantized>, u32)> + 'a {
        colors.iter().cloned().map(move |color| {
            let count = self.count_of(&color);
            (color, count)
        })
    }
}

struct Bounds(Rgb<MinMax<Quantized>>);

enum Dimension {
    R,
    G,
    B,
}

impl Bounds {
    fn new(color: Rgb<Quantized>) -> Self {
        Self(color.map(MinMax::from_value))
    }

    fn extend(&mut self, color: Rgb<Quantized>) {
        self.0.as_mut().zip(color).map(|(mm, c)| mm.extend(c));
    }

    fn volume(&self) -> usize {
        self.0.r.len() * self.0.g.len() * self.0.b.len()
    }

    fn longest_dimension(&self) -> Dimension {
        let r = self.0.r.len();
        let g = self.0.g.len();
        let b = self.0.b.len();
        if r >= g && r >= b {
            Dimension::R
        } else if g >= r && g >= b {
            Dimension::G
        } else {
            Dimension::B
        }
    }
}

struct VBox<'a> {
    bounds: Bounds,
    colors: &'a mut [Rgb<Quantized>],
    population: u32,
}

impl<'a> VBox<'a> {
    fn from_colors(colors: &'a mut [Rgb<Quantized>], histogram: &Histogram) -> Self {
        debug_assert_ne!(colors.len(), 0);
        let mut iter = histogram.colors(colors);
        let (first_color, first_count) = iter.next().unwrap();
        let mut bounds = Bounds::new(first_color);
        let mut population = first_count;
        for (color, count) in iter {
            bounds.extend(color);
            population += count;
        }
        Self {
            bounds,
            colors,
            population,
        }
    }

    fn average(&self, histogram: &Histogram) -> Color {
        let init = Rgb::<usize>::default();
        dbg!(&self.colors);
        let color = histogram
            .colors(self.colors)
            .fold(init, |acc_c, (v_c, v_p)| {
                let color = acc_c
                    .zip(v_c)
                    .map(|(a, b)| a + v_p as usize * b.to_color() as usize);
                color
            });
        dbg!(self.population);
        let color = color
            .map(|c| ((c as f64 / self.population as f64).round() as u8))
            .into_image_rgb();
        dbg!(color);
        Color {
            color,
            population: self.population as usize,
        }
    }

    fn volume(&self) -> usize {
        self.bounds.volume()
    }

    fn split(self, histogram: &Histogram) -> (VBox<'a>, Option<VBox<'a>>) {
        match self.bounds.longest_dimension() {
            Dimension::R => self
                .colors
                .sort_unstable_by(|a, b| [a.r, a.g, a.b].cmp(&[b.r, b.g, b.b])),
            Dimension::G => self
                .colors
                .sort_unstable_by(|a, b| [a.g, a.r, a.b].cmp(&[b.g, b.r, b.b])),
            Dimension::B => self
                .colors
                .sort_unstable_by(|a, b| [a.b, a.r, a.g].cmp(&[b.b, b.r, b.g])),
        }

        let split_point_population = self.population / 2;
        // dbg!(self.population, split_point_population);
        // Split after a sum of `split_point_population`, the first partition must not be empty and the last if possible neither
        let split_point = self
            .colors
            .iter()
            .position(|c| split_point_population <= histogram.count_of(c))
            .map(|v| (v + 1))
            .unwrap_or(self.colors.len())
            .min(self.colors.len() - 1)
            .max(1);
        // dbg!(split_point, self.colors.len());
        let (a, b) = self.colors.split_at_mut(split_point);
        let a = VBox::from_colors(a, histogram);
        let b = Some(b)
            .filter(|c| !c.is_empty())
            .map(|c| VBox::from_colors(c, histogram));
        (a, b)
    }
}

trait Box: Ord + Sized {
    fn split(self, histogram: &Histogram) -> (Self, Option<Self>);
}

trait Extractor {
    fn extract(vbox: &VBox) -> usize;
}

struct PopulationVolumeExtractor {}

impl Extractor for PopulationVolumeExtractor {
    fn extract(vbox: &VBox) -> usize {
        vbox.population as usize * vbox.volume()
    }
}

struct PopulationExtractor {}

impl Extractor for PopulationExtractor {
    fn extract(vbox: &VBox) -> usize {
        vbox.population as usize
    }
}

#[repr(transparent)]
struct SortedVBox<'a, E> {
    vbox: VBox<'a>,
    _marker: PhantomData<E>,
}

impl<'a, E> SortedVBox<'a, E> {
    fn new(vbox: VBox<'a>) -> Self {
        Self {
            vbox,
            _marker: PhantomData,
        }
    }
}

impl<'a, E: Extractor> SortedVBox<'a, E> {
    fn extract(&self) -> usize {
        E::extract(&self.vbox)
    }
}

impl<'a, E: Extractor> PartialEq for SortedVBox<'a, E> {
    fn eq(&self, other: &Self) -> bool {
        self.extract() == other.extract()
    }
}

impl<'a, E: Extractor> Eq for SortedVBox<'a, E> {}

impl<'a, E: Extractor> PartialOrd for SortedVBox<'a, E> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a, E: Extractor> Ord for SortedVBox<'a, E> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.extract().cmp(&other.extract())
    }
}

impl<'a, E: Extractor> Box for SortedVBox<'a, E> {
    fn split(self, histogram: &Histogram) -> (Self, Option<Self>) {
        let (a, b) = self.vbox.split(histogram);
        (Self::new(a), b.map(Self::new))
    }
}

fn split_boxes(queue: &mut BinaryHeap<impl Box>, histogram: &Histogram, target: usize) {
    debug_assert_ne!(target, 0);
    while queue.len() < target {
        let vbox = queue.pop().unwrap();
        let (vbox1, vbox2) = vbox.split(histogram);
        queue.push(vbox1);
        if let Some(vbox2) = vbox2 {
            queue.push(vbox2);
        } else {
            // Split didn't happen
            break;
        }
    }
}

/// Median cut quantizer
#[derive(Debug, Default)]
pub struct MedianCut;

const COLOR_RANGE: Range<usize> = 2..257;

impl Quantizer for MedianCut {
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
        if !COLOR_RANGE.contains(&colors) {
            return Err(Error::ColorCountOutOfBounds(colors, COLOR_RANGE));
        }

        let image = {
            let factor = 1.0 / quality as f64;
            let width = (image.width() as f64 * factor).round() as u32;
            let height = (image.height() as f64 * factor).round() as u32;
            resize(image, width, height, FilterType::Lanczos3)
        };
        let (histogram, mut distinct_colors) =
            Histogram::from_image(image.pixels().map(|p| p.to_rgba()), filter);
        // let mut v = histogram
        //     .buckets()
        //     .filter(|(_, count)| count != &0)
        //     .map(|(c, count)| (c.as_color_index(), count))
        //     .collect::<Vec<_>>();
        // v.sort_unstable_by_key(|&(_, c)| Reverse(c));
        // dbg!(image.len());
        // dbg!(v.len());
        // for (c, count) in v {
        //     println!("{}: {}", count, c);
        // }

        let vbox = VBox::from_colors(&mut distinct_colors, &histogram);
        let mut queue = BinaryHeap::new();
        queue.push(SortedVBox::<PopulationExtractor>::new(vbox));
        split_boxes(&mut queue, &histogram, (0.75 * colors as f64) as usize);
        let (slice, len, cap) = {
            let mut me = ManuallyDrop::new(queue.into_vec());
            (me.as_mut_ptr(), me.len(), me.capacity())
        };
        let vec = unsafe {
            Vec::from_raw_parts(
                slice as *mut SortedVBox<PopulationVolumeExtractor>,
                len,
                cap,
            )
        };
        let mut queue = BinaryHeap::from(vec);
        split_boxes(&mut queue, &histogram, colors);

        Ok(queue.iter().map(|b| b.vbox.average(&histogram)).collect())
    }
}
