use std::fmt;
use std::collections::BTreeMap;

use image::{GenericImage, Pixel, Rgb};

use hsl::HSL;
use crate::settings;
use crate::palette::Palette;

/// Vibrancy
///
/// 6 vibrant colors: primary, dark, light, dark muted and light muted.
#[derive(Debug, Hash, PartialEq, Eq, Default)]
pub struct Vibrancy {
    primary: Option<VibrancyColor>,
    dark: Option<VibrancyColor>,
    light: Option<VibrancyColor>,
    muted: Option<VibrancyColor>,
    dark_muted: Option<VibrancyColor>,
    light_muted: Option<VibrancyColor>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct VibrancyColor {
    color: Rgb<u8>,
    population: usize,
}

impl Vibrancy {
    /// Create new vibrancy map from an image
    pub fn new<P, G>(image: &G) -> Vibrancy
        where P: Sized + Pixel<Subpixel=u8>,
              G: Sized + GenericImage<Pixel=P>
    {
        generate_varation_colors(&Palette::new(image, 256, 10))
    }

    fn color_already_set(&self, color: &Rgb<u8>) -> bool {
        let color = Some(*color);

        // <option>.contains(color) does exactly this, but is marked as unstable.
        fn check_color(vibrancy_color_option: &Option<VibrancyColor>, expected: Option<Rgb<u8>>) -> bool {
            match vibrancy_color_option {
                Some(vibrancy_color) => Some(vibrancy_color.color) == expected,
                None => false
            }
        }

        check_color(&self.primary, color)
            || check_color(&self.dark, color)
            || check_color(&self.light, color)
            || check_color(&self.muted, color)
            || check_color(&self.dark_muted, color)
            || check_color(&self.light_muted, color)
    }

    fn find_color_variation(&self,
                            palette: &[Rgb<u8>],
                            pixel_counts: &BTreeMap<usize, usize>,
                            luma: &MTM<f64>,
                            saturation: &MTM<f64>)
                            -> Option<VibrancyColor> {
        let mut max = None;
        let mut max_value = 0_f64;

        let complete_population = pixel_counts.values().fold(0, |acc, c| acc + c);

        for (index, swatch) in palette.iter().enumerate() {
            let HSL { h: _, s, l } = HSL::from_rgb(swatch.channels());

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
                                                    complete_population as f64);
                if max.is_none() || value > max_value {
                    max = Some(VibrancyColor { color: swatch.clone(), population: population as usize });
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

    pub fn primary(&self) -> Option<VibrancyColor> {
        self.primary
    }

    pub fn dark(&self) -> Option<VibrancyColor> {
        self.dark
    }

    pub fn light(&self) -> Option<VibrancyColor> {
        self.light
    }

    pub fn muted(&self) -> Option<VibrancyColor> {
        self.muted
    }

    pub fn dark_muted(&self) -> Option<VibrancyColor> {
        self.dark_muted
    }

    pub fn light_muted(&self) -> Option<VibrancyColor> {
        self.light_muted
    }
}

impl fmt::Display for VibrancyColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rgb = self.color.channels();
        write!(
            f,
            "#{:02X}{:02X}{:02X}",
            rgb[0], rgb[1], rgb[2]
        )?;

        write!(f, ", {} pixels", self.population)
    }
}

impl fmt::Display for Vibrancy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Vibrant Colors {{\n")?;

        macro_rules! display_color {
            ($formatter:expr, $name:expr, $color:expr) => {
                write!($formatter, "\t")?;
                write!($formatter, $name)?;
                write!($formatter, ": ")?;
                if let Some(c) = $color {
                    write!($formatter, "{}", c)?;
                } else {
                    write!($formatter, "None")?;
                }

                write!($formatter, "\n")?;
            };
        }

        display_color!(f, "Primary Vibrant", self.primary);
        display_color!(f, "Dark Vibrant", self.dark);
        display_color!(f, "Light Vibrant", self.light);
        display_color!(f, "Muted", self.muted);
        display_color!(f, "Dark Muted", self.dark_muted);
        display_color!(f, "Light Muted", self.light_muted);

        write!(f, "}}")
    }
}

fn generate_varation_colors(p: &Palette) -> Vibrancy {
    let mut vibrancy = Vibrancy::default();
    vibrancy.primary = vibrancy.find_color_variation(
        &p.palette,
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
        }
    );

    vibrancy.light = vibrancy.find_color_variation(
        &p.palette,
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
        },
    );

    vibrancy.dark = vibrancy.find_color_variation(
        &p.palette,
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
        },
    );

    vibrancy.muted = vibrancy.find_color_variation(
        &p.palette,
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
        },
    );

    vibrancy.light_muted = vibrancy.find_color_variation(
        &p.palette,
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
        },
    );

    vibrancy.dark_muted = vibrancy.find_color_variation(
        &p.palette,
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
        }
    );

    vibrancy
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
