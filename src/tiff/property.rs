use super::{
    data::{Data, Tag, Tiff, IFD},
    tag::*,
    ParseTiffError,
};

#[derive(Debug)]
pub(crate) struct TiledIfd {
    pub width: u64,
    pub height: u64,
    pub tile_width: u64,
    pub tile_height: u64,
    pub offsets: Vec<u64>,
    pub byte_counts: Vec<u64>,
}

pub(crate) fn tiled_ifd<'a>(ifd: &IFD) -> Option<TiledIfd> {
    let width = ifd.get(&ImageWidth).and_then(to_u64)?;
    let height = ifd.get(&ImageWidth).and_then(to_u64)?;
    let tile_width = ifd.get(&TileWidth).and_then(to_u64)?;
    let tile_height = ifd.get(&TileLength).and_then(to_u64)?;
    let offsets = ifd.get(&TileOffsets).and_then(u64vec)?;
    let byte_counts = ifd.get(&TileByteCounts).and_then(u64vec)?;
    Some(TiledIfd {
        width,
        height,
        tile_width,
        tile_height,
        offsets,
        byte_counts,
    })
}

fn u8vec(d: &Data) -> Option<&Vec<u8>> {
    match d {
        Data::UndefinedVec(v) => Some(v),
        Data::ByteVec(v) => Some(v),
        _ => None,
    }
}

fn u64vec(d: &Data) -> Option<Vec<u64>> {
    match d {
        Data::LongVec(v) => Some(v.iter().map(|&x| x as u64).collect()),
        Data::Long8Vec(v) => Some(v.clone()),
        _ => None,
    }
}
fn to_u64(d: &Data) -> Option<u64> {
    match d {
        Data::Undefined(b) => Some(*b as u64),
        Data::Byte(b) => Some(*b as u64),
        Data::Short(b) => Some(*b as u64),
        Data::Long(b) => Some(*b as u64),
        Data::Long8(b) => Some(*b),
        _ => None,
    }
}
