use crate::tiff::decoder;
use crate::tiff::types::{KnownTag, IFD};
use std::fs::File;

pub(crate) fn level_count(file: File) -> u64 {
    let mut c = 0;
    match decoder::decode_file(file) {
        Ok(tiff) => {
            for d in tiff.iter() {
                if has_tiled_image(d) {
                    c += 1;
                }
            }
            c
        }
        Err(e) => {
            println!("Error while calling level_count {:?}", e);
            0
        }
    }
}

fn has_tiled_image(d: &IFD) -> bool {
    d.contains_key(&(KnownTag::TileOffsets as u16))
}
