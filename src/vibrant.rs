use std::fmt;
use std::collections::HashMap;

use image::{DynamicImage, Pixel, Rgb};

#[cfg(feature = "print-truecolor")]
use termion::color;

use hsl::HSL;
use crate::settings;
use crate::palette::Palette;

/// Vibrancy
///
/// 6 vibrant colors: primary, dark, light, dark muted and light muted.
#[derive(Debug, Hash, PartialEq, Eq, Default)]
pub struct Vibrancy {
    primary: Option<Rgb<u8>>,
    dark: Option<Rgb<u8>>,
    light: Option<Rgb<u8>>,
    muted: Option<Rgb<u8>>,
    dark_muted: Option<Rgb<u8>>,
    light_muted: Option<Rgb<u8>>,
}

impl Vibrancy {
    /// Create new vibrancy map from an image
    pub fn new(image: &DynamicImage) -> Vibrancy
    {
        let mut v = generate_varation_colors(&Palette::new(image, 64, 5));
        generate_empty_swatches(&mut v);
        v
    }

    fn color_already_set(&self, color: &Rgb<u8>) -> bool {
        let color = Some(*color);
        self.primary == color || self.dark == color || self.light == color ||
        self.muted == color || self.dark_muted == color || self.light_muted == color
    }

    fn find_color_variation(&self,
                            palette: &[Rgb<u8>],
                            pixel_counts: &HashMap<usize, usize>,
                            luma: &MTM<f64>,
                            saturation: &MTM<f64>)
                            -> Option<Rgb<u8>> {
        let mut max = None;
        let mut max_value = 0_f64;

        let complete_population = pixel_counts.values().max().unwrap();

        for (index, swatch) in palette.iter().enumerate() {
            let HSL {h: _, s, l} = HSL::from_rgb(swatch.channels());

            if s >= saturation.min && s <= saturation.max && l >= luma.min && l <= luma.max &&
               !self.color_already_set(swatch) {
                let population = *pixel_counts.get(&index).unwrap_or(&0) as f64;
                if population == 0_f64 {
                    continue;
                }
                let value = create_comparison_value(s,
                                                    saturation.target,
                                                    l,
                                                    luma.target,
                                                    population,
                                                    *complete_population as f64);
                if max.is_none() || value > max_value {
                    max = Some(swatch.clone());
                    max_value = value;
                }
            }
        }

        max
    }

    // fn fill_empty_swatches(self) {
    //     if self.primary.is_none() {
    //         // If we do not have a vibrant color...
    //         if let Some(dark) = self.dark {
    //             // ...but we do have a dark vibrant, generate the value by modifying the luma
    //             let hsl = HSL::from_pixel(&dark).clone()
    //             hsl.l = settings::TARGET_NORMAL_LUMA;
    //         }
    //     }
    // }
}

impl fmt::Display for Vibrancy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Vibrant Colors {{\n")?;

        macro_rules! display_color {
            ($formatter:expr, $name:expr, $color:expr) => {
                {
                    write!($formatter, "\t")?;
                    write!($formatter, $name)?;
                    if let Some(c) = $color {
                        let rgb = c.channels();
                        write!($formatter,
                            " Color: #{:02X}{:02X}{:02X}\t",
                            rgb[0], rgb[1], rgb[2]
                        )?;
                        #[cfg(feature = "print-truecolor")]
                        write!($formatter, "{}███{}\n", color::Fg(color::Rgb(rgb[0], rgb[1], rgb[2])), color::Fg(color::Reset))?;
                    } else {
                        write!($formatter, " Color: None\n")?;
                    }
                }
            };
        }

        display_color!(f, "Primary Vibrant", self.primary);
        display_color!(f, "Light Vibrant", self.light);
        display_color!(f, "Dark Vibrant", self.dark);
        display_color!(f, "Muted", self.muted);
        display_color!(f, "Light Muted", self.light_muted);
        display_color!(f, "Dark Muted", self.dark_muted);

        write!(f, "}}")
    }
}

fn generate_varation_colors(p: &Palette) -> Vibrancy {
    let mut vibrancy = Vibrancy::default();
    vibrancy.primary =
        vibrancy.find_color_variation(&p.palette,
                                      &p.pixel_counts,
                                      &MTM {
                                          min: settings::MIN_NORMAL_LUMA,
                                          target: settings::TARGET_NORMAL_LUMA,
                                          max: settings::MAX_NORMAL_LUMA,
                                      },
                                      &MTM {
                                          min: settings::MIN_VIBRANT_SATURATION,
                                          target: settings::TARGET_VIBRANT_SATURATION,
                                          max: 1_f64,
                                      });

    vibrancy.light = vibrancy.find_color_variation(&p.palette,
                                                   &p.pixel_counts,
                                                   &MTM {
                                                       min: settings::MIN_LIGHT_LUMA,
                                                       target: settings::TARGET_LIGHT_LUMA,
                                                       max: 1_f64,
                                                   },
                                                   &MTM {
                                                       min: settings::MIN_VIBRANT_SATURATION,
                                                       target: settings::TARGET_VIBRANT_SATURATION,
                                                       max: 1_f64,
                                                   });

    vibrancy.dark = vibrancy.find_color_variation(&p.palette,
                                                  &p.pixel_counts,
                                                  &MTM {
                                                      min: 0_f64,
                                                      target: settings::TARGET_DARK_LUMA,
                                                      max: settings::MAX_DARK_LUMA,
                                                  },
                                                  &MTM {
                                                      min: settings::MIN_VIBRANT_SATURATION,
                                                      target: settings::TARGET_VIBRANT_SATURATION,
                                                      max: 1_f64,
                                                  });

    vibrancy.muted = vibrancy.find_color_variation(&p.palette,
                                                   &p.pixel_counts,
                                                   &MTM {
                                                       min: settings::MIN_NORMAL_LUMA,
                                                       target: settings::TARGET_NORMAL_LUMA,
                                                       max: settings::MAX_NORMAL_LUMA,
                                                   },
                                                   &MTM {
                                                       min: 0_f64,
                                                       target: settings::TARGET_MUTED_SATURATION,
                                                       max: settings::MAX_MUTED_SATURATION,
                                                   });

    vibrancy.light_muted = vibrancy.find_color_variation(&p.palette,
                                                         &p.pixel_counts,
                                                         &MTM {
                                                             min: settings::MIN_LIGHT_LUMA,
                                                             target: settings::TARGET_LIGHT_LUMA,
                                                             max: 1_f64,
                                                         },
                                                         &MTM {
            min: 0_f64,
            target: settings::TARGET_MUTED_SATURATION,
            max: settings::MAX_MUTED_SATURATION,
        });

    vibrancy.dark_muted = vibrancy.find_color_variation(&p.palette,
                                                        &p.pixel_counts,
                                                        &MTM {
                                                            min: 0_f64,
                                                            target: settings::TARGET_DARK_LUMA,
                                                            max: settings::MAX_DARK_LUMA,
                                                        },
                                                        &MTM {
            min: 0_f64,
            target: settings::TARGET_MUTED_SATURATION,
            max: settings::MAX_MUTED_SATURATION,
        });

    vibrancy
}

fn generate_empty_swatches(v: &mut Vibrancy) {
    if v.primary.is_none() && v.dark.is_none() && v.light.is_none() {
        if v.dark.is_none() && v.dark_muted.is_some() {
            let mut hsl = HSL::from_rgb(v.dark_muted.unwrap().channels());
            hsl.l = settings::TARGET_DARK_LUMA;
            let (r, g, b) = hsl.to_rgb();
            v.dark = Some(Rgb::<u8>([r, g, b]));
        }
        if v.light.is_none() && v.light_muted.is_some() {
            let mut hsl = HSL::from_rgb(v.light_muted.unwrap().channels());
            hsl.l = settings::TARGET_LIGHT_LUMA;
            let (r, g, b) = hsl.to_rgb();
            v.light = Some(Rgb::<u8>([r, g, b]));
        }
    }
    if v.primary.is_none() && v.dark.is_some() {
        let mut hsl = HSL::from_rgb(v.dark.unwrap().channels());
        hsl.l = settings::TARGET_NORMAL_LUMA;
        let (r, g, b) = hsl.to_rgb();
        v.primary = Some(Rgb::<u8>([r, g, b]));
    } else if v.primary.is_none() && v.light.is_some() {
        let mut hsl = HSL::from_rgb(v.light.unwrap().channels());
        hsl.l = settings::TARGET_NORMAL_LUMA;
        let (r, g, b) = hsl.to_rgb();
        v.primary = Some(Rgb::<u8>([r, g, b]));
    }
    if v.dark.is_none() && v.primary.is_some() {
        let mut hsl = HSL::from_rgb(v.primary.unwrap().channels());
        hsl.l = settings::TARGET_DARK_LUMA;
        let (r, g, b) = hsl.to_rgb();
        v.dark = Some(Rgb::<u8>([r, g, b]));
    }
    if v.light.is_none() && v.primary.is_some() {
        let mut hsl = HSL::from_rgb(v.primary.unwrap().channels());
        hsl.l = settings::TARGET_LIGHT_LUMA;
        let (r, g, b) = hsl.to_rgb();
        v.light = Some(Rgb::<u8>([r, g, b]));
    }
    if v.muted.is_none() && v.primary.is_some() {
        let mut hsl = HSL::from_rgb(v.primary.unwrap().channels());
        hsl.s = settings::TARGET_MUTED_SATURATION;
        let (r, g, b) = hsl.to_rgb();
        v.muted = Some(Rgb::<u8>([r, g, b]));
    }
    if v.dark_muted.is_none() && v.dark.is_some() {
        let mut hsl = HSL::from_rgb(v.dark.unwrap().channels());
        hsl.s = settings::TARGET_MUTED_SATURATION;
        let (r, g, b) = hsl.to_rgb();
        v.dark_muted = Some(Rgb::<u8>([r, g, b]));
    }
    if v.light_muted.is_none() && v.light.is_some() {
        let mut hsl = HSL::from_rgb(v.light.unwrap().channels());
        hsl.s = settings::TARGET_MUTED_SATURATION;
        let (r, g, b) = hsl.to_rgb();
        v.light_muted = Some(Rgb::<u8>([r, g, b]));
    }
}

fn invert_diff(val: f64, target_val: f64) -> f64 {
    1_f64 - (val - target_val).abs()
}

fn weighted_mean(vals: &[(f64, f64)]) -> f64 {
    let (sum, sum_weight) = vals.iter().fold((0_f64, 0_f64),
                                             |(sum, sum_weight), &(val, weight)| {
                                                 (sum + val * weight, sum_weight + weight)
                                             });

    sum / sum_weight
}

fn create_comparison_value(sat: f64,
                           target_sat: f64,
                           luma: f64,
                           target_uma: f64,
                           population: f64,
                           max_population: f64)
                           -> f64 {
    weighted_mean(&[(invert_diff(sat, target_sat),
                     settings::WEIGHT_SATURATION),
                    (invert_diff(luma, target_uma), settings::WEIGHT_LUMA),
                    (population / max_population,
                     settings::WEIGHT_POPULATION)])
}

/// Minimum, Maximum, Target
#[derive(Debug, Hash)]
struct MTM<T> {
    min: T,
    target: T,
    max: T,
}
