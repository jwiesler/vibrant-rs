use std::collections::HashMap;
use priority_queue::PriorityQueue;
use image::{Pixel, Rgb};
use hsl::HSL;

use crate::palette::Swatch;

const BLACK_MAX_LIGHTNESS: f64 = 0.05;
const WHITE_MIN_LIGHTNESS: f64 = 0.95;

///An color quantizer based on the Median-cut algorithm, but optimized for picking out distinct
///colors rather than representation colors.
///
///The color space is represented as a 3-dimensional cube with each dimension being an RGB
///component. The cube is then repeatedly divided until we have reduced the color space to the
///requested number of colors. An average color is then generated from each cube.
///
///What makes this different to median-cut is that median-cut divided cubes so that all of the cubes
///have roughly the same population, where this quantizer divides boxes based on their color volume.
///This means that the color space is divided into distinct colors, rather than representative
///colors.
/*
pub struct Quantizer {
    pub colors: Vec<Rgb<u8>>,
    pub color_pop: HashMap<usize, usize>,
    pub quantized_colors: Vec<Rgb<u8>>,
}
*/

#[derive(Debug, Hash, PartialEq, Eq)]
struct Vbox {
    lower_index: usize,
    upper_index: usize,
    min_red: u8,
    max_red: u8,
    min_green: u8,
    max_green: u8,
    min_blue: u8,
    max_blue: u8,
}

enum ColorChannel {
   Red,
   Green,
   Blue,
}

impl Vbox {
    pub fn new(lower_index: usize, upper_index: usize, colors: &Vec<Rgb<u8>>) -> Vbox {
        let mut vbox = Vbox {lower_index, upper_index, min_red: 0xff, min_green: 0xff, min_blue: 0xff, max_red: 0, max_green: 0,
                 max_blue: 0};
        vbox.fit_box(colors);
        vbox
    }
    pub fn get_volume(&self) -> u32 {
        (self.max_red as u32 - self.min_red as u32 + 1) * (self.max_green as u32- self.min_green as u32+ 1) *
                (self.max_blue as u32 - self.min_blue as u32 + 1)
    }
    fn can_split(&self) -> bool {
        self.get_color_count() > 1
    }
    fn get_color_count(&self) -> usize {
        self.upper_index - self.lower_index + 1
    }
    fn fit_box(&mut self, colors: &Vec<Rgb<u8>>) {
        self.min_red = 0xff;
        self.min_green = 0xff;
        self.min_blue = 0xff;
        self.max_red = 0x0;
        self.max_green = 0x0;
        self.max_blue = 0x0;

        for i in self.lower_index..self.upper_index + 1 {
            let color = colors[i];
            if color[0] > self.max_red { self.max_red = color[0] };
            if color[1] > self.max_green { self.max_green = color[1] };
            if color[2] > self.max_blue { self.max_blue = color[2] };
            if color[0] < self.min_red { self.min_red = color[0] };
            if color[1] < self.min_green { self.min_green = color[1] };
            if color[2] < self.min_blue { self.min_blue = color[2] };
        }
    }
    pub fn split_box(&mut self, colors: &mut Vec<Rgb<u8>>) -> Vbox {
        if !self.can_split() { panic!("Cannot split a box with only 1 color") };
        let split_point = self.find_split_point(colors);
        let new_box = Vbox::new(split_point + 1, self.upper_index, colors);
        self.upper_index = split_point;
        self.fit_box(colors);
        new_box
    }
    fn get_longest_color_dimension(&self) -> ColorChannel {
        let red_len = self.max_red - self.min_red;
        let green_len = self.max_green - self.min_green;
        let blue_len = self.max_blue - self.min_blue;
        if red_len >= green_len && red_len >= blue_len {
            ColorChannel::Red
        } else if green_len >= red_len && green_len >= blue_len {
            ColorChannel::Green
        } else {
            ColorChannel::Blue
        }
    }
    fn find_split_point(&self, colors: &mut Vec<Rgb<u8>>) -> usize {
        let longest_dim = self.get_longest_color_dimension();
        let col_slice = &mut colors[self.lower_index..self.upper_index + 1];
        col_slice.sort_by(|a, b| {
            match longest_dim {
                ColorChannel::Red => a[0].cmp(&b[0]),
                ColorChannel::Green => a[1].cmp(&b[0]),
                ColorChannel::Blue => a[2].cmp(&b[0]),
            }
        });
        let dim_midpoint = self.mid_point(&longest_dim);
        for i in self.lower_index..self.upper_index + 1 {
            let color = colors[i];
            match longest_dim {
                ColorChannel::Red => if color[0] >= dim_midpoint { return i },
                ColorChannel::Green => if color[1] >= dim_midpoint { return i },
                ColorChannel::Blue => if color[2] > dim_midpoint { return i },
            }
        }
        return self.lower_index;
    }
    fn get_average_color(&self, colors: &Vec<Rgb<u8>>, color_pops: &HashMap<Rgb<u8>, usize>) -> Swatch {
        let mut red_sum: usize = 0;
        let mut green_sum: usize = 0;
        let mut blue_sum: usize = 0;
        let mut total_pop: usize = 0;
        for i in self.lower_index..self.upper_index + 1 {
            let color = colors[i];
            let color_pop = color_pops.get(&color).unwrap();
            total_pop += *color_pop;
            red_sum += *color_pop * color[0] as usize;
            green_sum += *color_pop * color[1] as usize;
            blue_sum += *color_pop * color[2] as usize;
        }
        let red_avg = (red_sum as f32 / total_pop as f32).round() as u8;
        let green_avg = (green_sum as f32 / total_pop as f32).round() as u8;
        let blue_avg = (blue_sum as f32 / total_pop as f32).round() as u8;
        //println!("total pop: {}", total_pop);
        Swatch {rgb: Rgb::<u8>([red_avg, green_avg, blue_avg]), population: total_pop}
    }
    fn mid_point(&self, channel: &ColorChannel) -> u8 {
        match channel {
            ColorChannel::Red => (self.min_red + self.max_red) / 2,
            ColorChannel::Green => (self.min_green + self.max_green) / 2,
            ColorChannel::Blue => (self.min_blue + self.max_blue) / 2,
        }
    }
}
pub fn quantize_pixels (max_color_index: usize, max_colors: usize, colors: &mut Vec<Rgb<u8>>, color_pops: &HashMap<Rgb<u8>, usize>)
        -> Vec<Swatch> {
    // TODO use something else other than priority queue (binary_heap?)
    let mut pq = PriorityQueue::with_capacity(max_colors);
    let full_box = Vbox::new(0, max_color_index, colors);
    let vol = full_box.get_volume();
    pq.push(full_box, vol);
    split_boxes(&mut pq, max_colors, colors);
    generate_average_colors(pq.into_sorted_vec(), colors, color_pops)
}
fn split_boxes(queue: &mut PriorityQueue<Vbox,u32>, max_size: usize, colors: &mut Vec<Rgb<u8>>) {
    while queue.len() < max_size {
        let vbox = queue.pop();
        match vbox {
            Some((mut b,_)) => {
                let split_box = b.split_box(colors);
                let split_vol = split_box.get_volume();
                queue.push(split_box, split_vol);
                let b_vol = b.get_volume();
                queue.push(b, b_vol);
            }
            None => return,
        }
    }
}
fn generate_average_colors(vboxes: Vec<Vbox>, colors: &Vec<Rgb<u8>>, color_pops: &HashMap<Rgb<u8>, usize>) -> Vec<Swatch> {
    let mut avg_colors = Vec::<Swatch>::with_capacity(vboxes.len());
    for vbox in vboxes {
        let color = vbox.get_average_color(&colors, color_pops);
        if !should_ignore_color(color.rgb) {
            avg_colors.push(color);
        }
    }
    avg_colors
}
fn should_ignore_color(color: Rgb<u8>) -> bool {
    let hsl_color = HSL::from_rgb(color.channels());
    is_white(hsl_color) || is_black(hsl_color) || is_near_red_i_line(hsl_color)
}
fn is_black(color: HSL) -> bool {
    color.l <= BLACK_MAX_LIGHTNESS
}
fn is_white(color: HSL) -> bool {
    color.l >= WHITE_MIN_LIGHTNESS
}
fn is_near_red_i_line(color: HSL) -> bool {
    color.h >= 10.0 && color.h <= 37.0 && color.s <= 0.82
}
