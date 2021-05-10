use std::array::IntoIter;
use std::fmt;

use hsl::HSL;
use image::{Pixel, Rgb};

use crate::{settings, Color};

/// Vibrancy
///
/// 6 vibrant colors: primary, dark, light, dark muted and light muted.
#[derive(Debug, Hash, PartialEq, Eq, Default)]
pub struct Vibrancy {
    /// Primary vibrant color
    pub primary: Option<Color>,
    /// Dark vibrant color
    pub dark: Option<Color>,
    /// Light vibrant color
    pub light: Option<Color>,
    /// Muted vibrant color
    pub muted: Option<Color>,
    /// Dark muted vibrant color
    pub dark_muted: Option<Color>,
    /// Light muted vibrant color
    pub light_muted: Option<Color>,
}

impl Vibrancy {
    /// Create new vibrancy map from an image
    pub fn from_palette(palette: &[Color]) -> Vibrancy {
        let mut vibrancy = Vibrancy::default();
        let max_population = palette.iter().map(|c| c.population).max().unwrap();
        vibrancy.primary = vibrancy.find_color_variation(
            palette,
            &MinMaxTarget {
                min: settings::MIN_NORMAL_LUMA,
                target: settings::TARGET_NORMAL_LUMA,
                max: settings::MAX_NORMAL_LUMA,
            },
            &MinMaxTarget {
                min: settings::MIN_VIBRANT_SATURATION,
                target: settings::TARGET_VIBRANT_SATURATION,
                max: 1_f64,
            },
            max_population,
        );

        vibrancy.light = vibrancy.find_color_variation(
            palette,
            &MinMaxTarget {
                min: settings::MIN_LIGHT_LUMA,
                target: settings::TARGET_LIGHT_LUMA,
                max: 1_f64,
            },
            &MinMaxTarget {
                min: settings::MIN_VIBRANT_SATURATION,
                target: settings::TARGET_VIBRANT_SATURATION,
                max: 1_f64,
            },
            max_population,
        );

        vibrancy.dark = vibrancy.find_color_variation(
            palette,
            &MinMaxTarget {
                min: 0_f64,
                target: settings::TARGET_DARK_LUMA,
                max: settings::MAX_DARK_LUMA,
            },
            &MinMaxTarget {
                min: settings::MIN_VIBRANT_SATURATION,
                target: settings::TARGET_VIBRANT_SATURATION,
                max: 1_f64,
            },
            max_population,
        );

        vibrancy.muted = vibrancy.find_color_variation(
            palette,
            &MinMaxTarget {
                min: settings::MIN_NORMAL_LUMA,
                target: settings::TARGET_NORMAL_LUMA,
                max: settings::MAX_NORMAL_LUMA,
            },
            &MinMaxTarget {
                min: 0_f64,
                target: settings::TARGET_MUTED_SATURATION,
                max: settings::MAX_MUTED_SATURATION,
            },
            max_population,
        );

        vibrancy.light_muted = vibrancy.find_color_variation(
            palette,
            &MinMaxTarget {
                min: settings::MIN_LIGHT_LUMA,
                target: settings::TARGET_LIGHT_LUMA,
                max: 1_f64,
            },
            &MinMaxTarget {
                min: 0_f64,
                target: settings::TARGET_MUTED_SATURATION,
                max: settings::MAX_MUTED_SATURATION,
            },
            max_population,
        );

        vibrancy.dark_muted = vibrancy.find_color_variation(
            palette,
            &MinMaxTarget {
                min: 0_f64,
                target: settings::TARGET_DARK_LUMA,
                max: settings::MAX_DARK_LUMA,
            },
            &MinMaxTarget {
                min: 0_f64,
                target: settings::TARGET_MUTED_SATURATION,
                max: settings::MAX_MUTED_SATURATION,
            },
            max_population,
        );

        vibrancy
    }

    fn color_already_set(&self, color: &Rgb<u8>) -> bool {
        IntoIter::new([
            self.primary.as_ref(),
            self.dark.as_ref(),
            self.light.as_ref(),
            self.muted.as_ref(),
            self.dark_muted.as_ref(),
            self.light_muted.as_ref(),
        ])
        .any(|v| v.map(|c| &c.color) == Some(color))
    }

    fn find_color_variation(
        &self,
        palette: &[Color],
        luma: &MinMaxTarget<f64>,
        saturation: &MinMaxTarget<f64>,
        max_population: usize,
    ) -> Option<Color> {
        let mut max = None;
        let mut max_value = 0_f64;

        for &Color { color, population } in palette.iter() {
            let HSL { h: _, s, l } = HSL::from_rgb(color.channels());

            if population != 0
                && s >= saturation.min
                && s <= saturation.max
                && l >= luma.min
                && l <= luma.max
                && !self.color_already_set(&color)
            {
                let value = create_comparison_value(
                    s,
                    saturation.target,
                    l,
                    luma.target,
                    population as f64,
                    max_population as f64,
                );
                if max.is_none() || value > max_value {
                    max = Some(Color { color, population });
                    max_value = value;
                }
            }
        }

        max
    }
}

impl fmt::Display for Vibrancy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Vibrant Colors {{")?;

        macro_rules! display_color {
            ($formatter:expr, $name:expr, $color:expr) => {
                write!($formatter, "\t")?;
                write!($formatter, $name)?;
                write!($formatter, ": ")?;
                if let Some(c) = $color {
                    writeln!($formatter, "{}", c)?;
                } else {
                    writeln!($formatter, "None")?;
                }
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

fn invert_diff(val: f64, target_val: f64) -> f64 {
    1_f64 - (val - target_val).abs()
}

fn weighted_mean(values: &[(f64, f64)]) -> f64 {
    let (sum, sum_weight) = values
        .iter()
        .fold((0_f64, 0_f64), |(sum, sum_weight), &(val, weight)| {
            (sum + val * weight, sum_weight + weight)
        });

    sum / sum_weight
}

fn create_comparison_value(
    sat: f64,
    target_sat: f64,
    luma: f64,
    target_uma: f64,
    population: f64,
    max_population: f64,
) -> f64 {
    weighted_mean(&[
        (invert_diff(sat, target_sat), settings::WEIGHT_SATURATION),
        (invert_diff(luma, target_uma), settings::WEIGHT_LUMA),
        (population / max_population, settings::WEIGHT_POPULATION),
    ])
}

/// Minimum, Maximum, Target
#[derive(Debug, Hash)]
struct MinMaxTarget<T> {
    min: T,
    target: T,
    max: T,
}
