use std::cmp;
use std::collections::BTreeMap;
use priority_queue::PriorityQueue;
use image::{Pixel, Rgb};

use crate::palette::Swatch;

const FRACT_BY_POPULATIONS: f64 = 0.75;

const SIGBITS: usize = 5;
const RSHIFT: usize = 8 - SIGBITS;


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

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct Vbox {
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

enum CompareFn {
   Count,
   Volume,
}
impl Vbox {
    pub fn new(colors: &Vec<Rgb<u8>>, hist: &mut BTreeMap<usize, usize>) -> Vbox {
        let hn = 1 << (3 * SIGBITS);
        let mut vbox = Vbox {min_red: 0xff, min_green: 0xff, min_blue: 0xff, max_red: 0, max_green: 0,
                 max_blue: 0};

        for i in colors {
            // TODO rewrite with let c = i.map(|a| a >> RSHIFT);
            let r = i[0] >> RSHIFT;
            let g = i[1] >> RSHIFT;
            let b = i[2] >> RSHIFT;

            let idx = get_color_index(Rgb::<u8>([r, g, b]));
            *hist.entry(idx).or_insert(0) += 1;
            if r > vbox.max_red { vbox.max_red = r };
            if g > vbox.max_green { vbox.max_green = g };
            if b > vbox.max_blue { vbox.max_blue = b };
            if r < vbox.min_red { vbox.min_red = r };
            if g < vbox.min_green { vbox.min_green = g };
            if b < vbox.min_blue { vbox.min_blue = b };
        }
        vbox
    }
    pub fn get_volume(&self) -> u32 {
        (self.max_red as u32 - self.min_red as u32 + 1) * (self.max_green as u32- self.min_green as u32+ 1) *
                (self.max_blue as u32 - self.min_blue as u32 + 1)
    }
    pub fn get_count(&self, hist: &BTreeMap<usize, usize>) -> u32 {
        let mut ct = 0;
        for (r, g, b) in
                iproduct!(self.min_red..=self.max_red, self.min_green..=self.max_green, self.min_blue..=self.max_blue) {
            let idx = get_color_index(Rgb::<u8>([r, g, b]));
            ct += match hist.get(&idx) {
                Some(p) => *p,
                None => continue,
            };
        }
        ct as u32
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
    fn avg(&self, hist: &BTreeMap<usize, usize>) -> Rgb<u8> {
        let mult: u8 = 1 << RSHIFT;
        let mut red_sum: usize = 0;
        let mut green_sum: usize = 0;
        let mut blue_sum: usize = 0;
        let mut total_pop: usize = 0;

        for (r, g, b) in
                iproduct!(self.min_red..=self.max_red, self.min_green..=self.max_green, self.min_blue..=self.max_blue) {
            let idx = get_color_index(Rgb::<u8>([r, g, b]));
            match hist.get(&idx) {
                None => continue,
                Some(h) => {
                    total_pop += h;
                    red_sum += (*h as f32 * (r as f32 + 0.5) * mult as f32) as usize;
                    green_sum += (*h as f32 * (g as f32 + 0.5) * mult as f32) as usize;
                    blue_sum += (*h as f32 * (b as f32 + 0.5) * mult as f32) as usize;
                }
            }
        }

        if total_pop > 0 {
            Rgb::<u8>([(red_sum as f32 /total_pop as f32).floor() as u8,
                       (green_sum as f32 /total_pop as f32).floor() as u8,
                       (blue_sum as f32 /total_pop as f32).floor() as u8])
        } else {
            Rgb::<u8>([(mult as f32 * (self.min_red + self.max_red + 1) as f32/ 2.0).round() as u8,
                       (mult as f32 * (self.min_green + self.max_green + 1) as f32/ 2.0).round() as u8,
                       (mult as f32 * (self.min_blue + self.max_blue + 1) as f32/ 2.0).round() as u8])
        }
    }
    fn split(&mut self, hist: &BTreeMap<usize, usize>) -> Vbox {
        if self.get_count(hist) <= 1 { return self.clone() };
        let mut acc_sum = BTreeMap::<usize, usize>::new();
        let mut sum;
        let mut total = 0;
        let longest_dim = self.get_longest_color_dimension();
        match longest_dim {
            ColorChannel::Red => {
                for r in self.min_red..=self.max_red {
                    sum = 0;
                    for (g, b) in iproduct!(self.min_green..=self.max_green, self.min_blue..=self.max_blue) {
                        let idx = get_color_index(Rgb::<u8>([r, g, b]));
                        sum += match hist.get(&idx) { Some(a) => a, None => continue };
                    }
                    total += sum;
                    acc_sum.insert(r as usize, total);
                }
            }
            ColorChannel::Green => {
                for g in self.min_green..=self.max_green {
                    sum = 0;
                    for (r, b) in iproduct!(self.min_red..=self.max_red, self.min_blue..=self.max_blue) {
                        let idx = get_color_index(Rgb::<u8>([r, g, b]));
                        sum += match hist.get(&idx) { Some(a) => a, None => continue };
                    }
                    total += sum;
                    acc_sum.insert(g as usize, total);
                }
            }
            ColorChannel::Blue => {
                for b in self.min_blue..=self.max_blue {
                    sum = 0;
                    for (r, g) in iproduct!(self.min_red..=self.max_red, self.min_green..=self.max_green) {
                        let idx = get_color_index(Rgb::<u8>([r, g, b]));
                        sum += match hist.get(&idx) { Some(a) => a, None => continue };
                    }
                    total += sum;
                    acc_sum.insert(b as usize, total);
                }
            }
        }
        let (&splitpoint, _) = acc_sum.iter().find(|(_, &d)| d > total / 2).unwrap();

        let mut vbox2 = self.clone();
        match longest_dim {
            ColorChannel::Red => {
                let left = splitpoint - self.min_red as usize;
                let right = self.max_red as usize - splitpoint;
                if left <= right {
                    self.max_red = cmp::min(self.max_red - 1, (splitpoint as f32 + right as f32 / 2.0).round() as u8);
                    self.max_red = cmp::max(0, self.max_red);
                } else {
                    let tmp_max_red = cmp::max(self.min_red, (splitpoint as f32 - 1.0 - left as f32 / 2.0).round() as u8);
                    self.max_red = cmp::min(self.max_red, tmp_max_red);
                }
                self.max_red = *acc_sum.keys().find(|&k| *k >= self.max_red as usize).unwrap() as u8;
                vbox2.min_red = self.max_red + 1;
            }
            ColorChannel::Green => {
                let left = splitpoint - self.min_green as usize;
                let right = self.max_green as usize - splitpoint;
                if left <= right {
                    self.max_green = cmp::min(self.max_green - 1, (splitpoint as f32 + right as f32 / 2.0).round() as u8);
                    self.max_green = cmp::max(0, self.max_green);
                } else {
                    let tmp_max_green = cmp::max(self.min_green, (splitpoint as f32 - 1.0 - left as f32 / 2.0).round() as u8);
                    self.max_green = cmp::min(self.max_green, tmp_max_green);
                }
                self.max_green = *acc_sum.keys().find(|&k| *k >= self.max_green as usize).unwrap() as u8;
                vbox2.min_green = self.max_green + 1;
            }
            ColorChannel::Blue => {
                let left = splitpoint - self.min_blue as usize;
                let right = self.max_blue as usize - splitpoint;
                if left <= right {
                    self.max_blue = cmp::min(self.max_blue - 1, (splitpoint as f32 + right as f32 / 2.0).round() as u8);
                    self.max_blue = cmp::max(0, self.max_blue);
                } else {
                    let tmp_max_blue = cmp::max(self.min_blue, (splitpoint as f32 - 1.0 - left as f32 / 2.0).round() as u8);
                    self.max_blue = cmp::min(self.max_blue, tmp_max_blue);
                }
                self.max_blue = *acc_sum.keys().find(|&k| *k >= self.max_blue as usize).unwrap() as u8;
                vbox2.min_blue = self.max_blue + 1;
            }

        }
        vbox2
    }
}
pub fn quantize_pixels (max_colors: usize, colors: &mut Vec<Rgb<u8>>) -> Vec<Swatch> {
    // TODO use something else other than priority queue (binary_heap?)
    let mut pq = PriorityQueue::with_capacity(max_colors);
    let mut hist = BTreeMap::<usize, usize>::new();
    let full_box = Vbox::new(colors, &mut hist);
    let ct = full_box.get_count(&hist);
    pq.push(full_box, ct);
    // first set sorted by population
    split_boxes(&mut pq, (FRACT_BY_POPULATIONS * max_colors as f64) as usize, &hist, CompareFn::Count);

    // reorder the existing set by volume
    for (i,p) in pq.iter_mut() {
        *p = i.get_count(&hist) * i.get_volume();
    }

    // second set sorted by volume
    let sec_size = max_colors - pq.len();
    split_boxes(&mut pq, sec_size, &hist, CompareFn::Volume);
    generate_average_colors(pq.into_sorted_vec(), &hist)
}
fn split_boxes(queue: &mut PriorityQueue<Vbox,u32>, max_size: usize, hist: &BTreeMap<usize, usize>,  cmp: CompareFn) {
    let mut last_size = queue.len();
    while queue.len() < max_size {
        let vbox = queue.pop();
        match vbox {
            Some((mut b,_)) => {
                let split_box = b.split(hist);
                let split_val = match &cmp {
                    CompareFn::Volume => split_box.get_count(hist) * split_box.get_volume(),
                    CompareFn::Count => split_box.get_count(hist),
                };
                queue.push(split_box, split_val);
                let b_val = match &cmp {
                    CompareFn::Volume => b.get_count(hist) * b.get_volume(),
                    CompareFn::Count => b.get_count(hist),
                };
                queue.push(b, b_val);

                if queue.len() == last_size {
                    break;
                } else {
                    last_size = queue.len();
                }
            }
            None => return,
        }
    }
}
fn generate_average_colors(vboxes: Vec<Vbox>, hist: &BTreeMap<usize, usize>) -> Vec<Swatch> {
    let mut avg_colors = Vec::<Swatch>::with_capacity(vboxes.len());
    for vbox in vboxes {
        let color = vbox.avg(hist);
        avg_colors.push(Swatch{rgb: color, population: vbox.get_count(hist) as usize});
    }
    avg_colors
}
fn get_color_index(color: Rgb<u8>) -> usize {
    ((color[0] as usize) << (2 * SIGBITS)) + ((color[1] as usize) << SIGBITS) + color[2] as usize
}
