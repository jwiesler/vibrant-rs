extern crate image;
extern crate vibrant;

use std::env;
use std::path::Path;

use vibrant::Palette;

fn main() {
    let source = env::args().nth(1).expect("No source image given.");
    let img = image::open(&Path::new(&source))
        .unwrap_or_else(|_| panic!("Could not load image {:?}", source));

    println!("{}", Palette::new(&img, 10, 10).sort_by_frequency());
}
