use crate::tiff::tifflike::leica;
use std::fs::File;

pub fn is_leica(path: &str) -> bool {
    let file = File::open(path).unwrap();
    leica::check_compatible(file)
}
