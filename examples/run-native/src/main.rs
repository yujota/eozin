// Example for decoding Aperio file with std::io::Reader
//
// cargo run -p run-native
use eozin::std_io::DynamicDecoder;
use std::fs::{self, File};
use std::path::Path;

const SVS_LINK: &str = "https://openslide.cs.cmu.edu/download/openslide-testdata/Aperio/";
const TARGET_FILE: &str = "CMU-1-Small-Region.svs";

fn download_file() {
    if Path::new(TARGET_FILE).exists() {
        return;
    };
    println!(
        "File {} not found on the current directory. Start downloading..",
        TARGET_FILE
    );
    let url = SVS_LINK.to_string() + TARGET_FILE;
    let mut response = reqwest::blocking::get(&url).unwrap();
    let mut dest = File::create(TARGET_FILE).unwrap();
    let _ = std::io::copy(&mut response, &mut dest);
    println!("Download complete");
}

fn run_eozin() {
    let mut decoder = DynamicDecoder::with_path(TARGET_FILE).unwrap();
    println!("Slide Format: {}", decoder.slide_format());
    println!(
        "Slide Dimension: width {}, height {}",
        decoder.dimensions().0,
        decoder.dimensions().1
    );
    println!("Num levels: {}", decoder.level_count());
    let (lv, x, y) = (0, 12, 10);
    let buf = decoder.read_tile_as_bytes(lv, x, y).unwrap();
    let out_path = format!("tile-{}-{}-{}.jpg", lv, x, y);
    fs::write(out_path, &buf).unwrap();
}

fn main() {
    download_file();
    run_eozin();
}
