extern crate image;
extern crate vibrant;

use std::env;
use std::path::Path;

use vibrant::{Neu, Palette, Vibrancy};

fn main() {
    let source = env::args().nth(1).expect("No source image given.");
    let img = image::open(&Path::new(&source))
        .unwrap_or_else(|_| panic!("Could not load image {:?}", source));

    let palette = Palette::from_image(&img, 64, 10, &Neu::default()).unwrap();
    println!("{:#?}", Vibrancy::from_palette(&palette.palette));
}
