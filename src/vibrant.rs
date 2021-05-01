use std::array::IntoIter;
use std::collections::BTreeMap;
use std::fmt;

use hsl::HSL;
use image::{GenericImage, Pixel, Rgb};

use crate::palette::Palette;
use crate::settings;

/// Vibrancy
///
/// 6 vibrant colors: primary, dark, light, dark muted and light muted.
#[derive(Debug, Hash, PartialEq, Eq, Default)]
pub struct Vibrancy {
    /// Primary vibrant color
    pub primary: Option<VibrancyColor>,
    /// Dark vibrant color
    pub dark: Option<VibrancyColor>,
    /// Light vibrant color
    pub light: Option<VibrancyColor>,
    /// Muted vibrant color
    pub muted: Option<VibrancyColor>,
    /// Dark muted vibrant color
    pub dark_muted: Option<VibrancyColor>,
    /// Light muted vibrant color
    pub light_muted: Option<VibrancyColor>,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct VibrancyColor {
    pub color: Rgb<u8>,
    pub population: usize,
}

impl AsRef<Rgb<u8>> for VibrancyColor {
    fn as_ref(&self) -> &Rgb<u8> {
        &self.color
    }
}

impl Vibrancy {
    /// Create new vibrancy map from an image
    pub fn new<P, G>(image: &G) -> Vibrancy
    where
        P: Sized + Pixel<Subpixel = u8>,
        G: Sized + GenericImage<Pixel = P>,
    {
        generate_varation_colors(&Palette::new(image, 256, 10))
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
        .any(|v| v.map(AsRef::as_ref) == Some(color))
    }

    fn find_color_variation(
        &self,
        palette: &[Rgb<u8>],
        pixel_counts: &BTreeMap<usize, usize>,
        luma: &MinMaxTarget<f64>,
        saturation: &MinMaxTarget<f64>,
    ) -> Option<VibrancyColor> {
        let mut max = None;
        let mut max_value = 0_f64;

        let complete_population = pixel_counts.values().sum::<usize>();

        for (index, swatch) in palette.iter().enumerate() {
            let HSL { h: _, s, l } = HSL::from_rgb(swatch.channels());

            if s >= saturation.min
                && s <= saturation.max
                && l >= luma.min
                && l <= luma.max
                && !self.color_already_set(swatch)
            {
                let population = pixel_counts.get(&index).copied().unwrap_or(0) as f64;
                if population == 0_f64 {
                    continue;
                }
                let value = create_comparison_value(
                    s,
                    saturation.target,
                    l,
                    luma.target,
                    population,
                    complete_population as f64,
                );
                if max.is_none() || value > max_value {
                    max = Some(VibrancyColor {
                        color: *swatch,
                        population: population as usize,
                    });
                    max_value = value;
                }
            }
        }

        max
    }
}

impl fmt::Display for VibrancyColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let rgb = self.color.channels();
        write!(f, "#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])?;

        write!(f, ", {} pixels", self.population)
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

fn generate_varation_colors(p: &Palette) -> Vibrancy {
    let mut vibrancy = Vibrancy::default();
    vibrancy.primary = vibrancy.find_color_variation(
        &p.palette,
        &p.pixel_counts,
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
    );

    vibrancy.light = vibrancy.find_color_variation(
        &p.palette,
        &p.pixel_counts,
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
    );

    vibrancy.dark = vibrancy.find_color_variation(
        &p.palette,
        &p.pixel_counts,
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
    );

    vibrancy.muted = vibrancy.find_color_variation(
        &p.palette,
        &p.pixel_counts,
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
    );

    vibrancy.light_muted = vibrancy.find_color_variation(
        &p.palette,
        &p.pixel_counts,
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
    );

    vibrancy.dark_muted = vibrancy.find_color_variation(
        &p.palette,
        &p.pixel_counts,
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
    );

    vibrancy
}

fn invert_diff(val: f64, target_val: f64) -> f64 {
    1_f64 - (val - target_val).abs()
}

fn weighted_mean(vals: &[(f64, f64)]) -> f64 {
    let (sum, sum_weight) = vals
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
