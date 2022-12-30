use crate::tiff::tifflike::aperio;
use std::fs::File;

pub fn level_count(path: &str) -> u64 {
    let mut file = File::open(path).unwrap();
    aperio::level_count(file)
}
