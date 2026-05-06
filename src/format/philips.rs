use super::generic_tiff::{ReadTile, ReadTileInput};
use crate::base::{
    DecoderConstructor, EozinDecoderCore, ImageType, LevelInfo, LevelInfoHandler, MapOutput,
    OutputMapper, ReadTileDecoder,
};
use crate::error::EozinError::{self, *};
use crate::tiff_tools::{CommonTag::*, Ifd, TiffDecoder};

pub(crate) type PhilipsDecoderConstructor = MapOutput<TiffDecoder, IfdToDecoder>;

impl DecoderConstructor for PhilipsDecoderConstructor {}

pub(crate) type PhilipsReadTile = ReadTile;
pub(crate) type PhilipsReadTileInput = ReadTileInput;

pub(crate) struct IfdToDecoder {}

impl OutputMapper for IfdToDecoder {
    type Input = Vec<Ifd>;
    type Output = PhilipsDecoder;
    type ErrorKind = EozinError;
    fn map(input: Self::Input) -> Result<Self::Output, Self::ErrorKind> {
        let mut levels = Vec::new();
        for ifd in input {
            match Level::from_ifd(&ifd) {
                Some(lv) => levels.push(lv),
                _ => continue,
            }
        }
        Ok(PhilipsDecoder { levels })
    }
}

#[derive(Debug)]
pub struct PhilipsDecoder {
    levels: Vec<Level>,
}

impl PhilipsDecoder {
    pub fn with_ifds(dirs: &[Ifd]) -> Result<PhilipsDecoder, EozinError> {
        let mut levels = Vec::new();
        for ifd in dirs {
            match Level::from_ifd(ifd) {
                Some(lv) => levels.push(lv),
                _ => continue,
            }
        }
        Ok(PhilipsDecoder { levels })
    }
}

impl LevelInfoHandler for PhilipsDecoder {
    fn get_level(&self, i: usize) -> Option<LevelInfo> {
        self.levels.get(i).map(|l| l.info)
    }
    fn level_count(&self) -> usize {
        self.levels.len()
    }
    fn marginal_tile_size(&self, _lv: usize) -> Option<(u64, u64)> {
        None
    }
}

impl ReadTileDecoder for PhilipsDecoder {
    type ReadTile = ReadTile;
    type ReadTileInput = ReadTileInput;
    fn read_tile(&self, lv: usize, x: usize, y: usize) -> Result<ReadTileInput, EozinError> {
        let lv = self.levels.get(lv).ok_or(LevelNotFound {
            selected: lv,
            num_level: self.levels.len(),
        })?;
        let tile_id = lv.info.tile_range_x * y + x;
        let addr = lv.tile_offsets.get(tile_id);
        let count = lv.tile_byte_counts.get(tile_id);
        let (&addr, &count) = addr.zip(count).ok_or(TileNotFound { x, y, z: 0 })?;
        Ok(ReadTileInput {
            jpeg_tables: lv.jpeg_tables.clone(),
            offset: addr,
            len: count,
        })
    }
}

impl EozinDecoderCore for PhilipsDecoder {}

#[derive(Debug)]
struct Level {
    pub info: LevelInfo,
    pub tile_offsets: Vec<u64>,
    pub tile_byte_counts: Vec<u64>,
    pub jpeg_tables: Vec<u8>,
}

impl Level {
    fn from_ifd(ifd: &Ifd) -> Option<Self> {
        let width = ifd.get(ImageWidth).and_then(|d| d.to_u64())?;
        let height = ifd.get(ImageLength).and_then(|d| d.to_u64())?;
        let tile_width = ifd.get(TileWidth).and_then(|d| d.to_u64())?;
        let tile_height = ifd.get(TileLength).and_then(|d| d.to_u64())?;
        let tile_offsets = ifd.get(TileOffsets).and_then(|d| d.to_u64vec())?;
        let tile_byte_counts = ifd.get(TileByteCounts).and_then(|d| d.to_u64vec())?;
        let jpeg_tables = ifd.get(JPEGTables).and_then(|d| d.expect_u8vec())?;
        let compression = ifd.get(Compression).and_then(|d| d.to_u64())?;
        if compression != 7 {
            return None;
        }
        let _image_description_xml = ifd.get(ImageDescription).and_then(|d| d.expect_ascii())?;
        let software = ifd.get(Software).and_then(|d| d.expect_ascii())?;
        if !software.starts_with("Philips") {
            return None;
        }
        let info = LevelInfo {
            width,
            height,
            tile_width,
            tile_height,
            tile_range_x: diva(width, tile_width) as usize,
            tile_range_y: diva(height, tile_height) as usize,
            image_type: ImageType::Jpeg,
        };
        Some(Level {
            info,
            jpeg_tables,
            tile_byte_counts,
            tile_offsets,
        })
    }
}

fn diva(a: u64, b: u64) -> u64 {
    if a % b == 0 { a / b } else { a / b + 1 }
}
