use crate::base::{
    DecoderConstructor, EozinDecoderCore, LevelInfo, LevelInfoHandler, MapOutput, OutputMapper,
    ReadCommand, ReadConsumer, ReadTileDecoder, ImageType
};
use crate::error::EozinError::{self, *};
use crate::tiff_tools::{CommonTag::*, Ifd, TiffDecoder};

pub(crate) type GenericTiffDecoderConstructor = MapOutput<TiffDecoder, IfdToDecoder>;

impl DecoderConstructor for GenericTiffDecoderConstructor {}

pub(crate) struct IfdToDecoder {}

pub(crate) type GenericReadTile = ReadTile;
pub(crate) type GenericReadTileInput = ReadTileInput;

impl OutputMapper for IfdToDecoder {
    type Input = Vec<Ifd>;
    type Output = GenericTiffDecoder;
    type ErrorKind = EozinError;
    fn map(input: Self::Input) -> Result<Self::Output, Self::ErrorKind> {
        let mut levels = Vec::new();
        for ifd in input {
            match Level::from_ifd(&ifd) {
                Some(lv) => levels.push(lv),
                _ => continue,
            }
        }
        Ok(GenericTiffDecoder { levels })
    }
}

#[derive(Debug)]
pub struct GenericTiffDecoder {
    levels: Vec<Level>,
}

impl GenericTiffDecoder {
    pub fn with_ifds(dirs: &[Ifd]) -> Result<GenericTiffDecoder, EozinError> {
        let mut levels = Vec::new();
        for ifd in dirs {
            match Level::from_ifd(ifd) {
                Some(lv) => levels.push(lv),
                _ => continue,
            }
        }
        Ok(GenericTiffDecoder { levels })
    }
}


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
        let info = LevelInfo {
            width,
            height,
            tile_width,
            tile_height,
            tile_range_x: diva(width, tile_width) as usize,
            tile_range_y: diva(height, tile_height) as usize,
            image_type: ImageType::Jpeg
        };
        Some(Level {
            info,
            jpeg_tables,
            tile_byte_counts,
            tile_offsets,
        })
    }
}

impl LevelInfoHandler for GenericTiffDecoder {
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

impl ReadTileDecoder for GenericTiffDecoder {
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

#[derive(Debug, Default)]
pub(crate) struct ReadTileInput {
    pub jpeg_tables: Vec<u8>,
    pub offset: u64,
    pub len: u64,
}

#[derive(Debug)]
pub(crate) struct ReadTile {
    pub jpeg_tables: Vec<u8>,
}

impl ReadConsumer for ReadTile {
    type Input = ReadTileInput;
    type Output = Vec<u8>;
    type ErrorKind = EozinError;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand) {
        (
            ReadTile {
                jpeg_tables: input.jpeg_tables,
            },
            ReadCommand::ReadBytes {
                offset: input.offset,
                count: input.len,
            },
        )
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        let ReadTile {
            mut jpeg_tables, ..
        } = self;
        let _ = jpeg_tables.split_off(jpeg_tables.len() - 2);
        jpeg_tables.extend_from_slice(&buf[2..]);
        Ok(jpeg_tables)
    }
    fn step(&mut self, _buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        Err(UnexpectedStep)
    }
}

impl EozinDecoderCore for GenericTiffDecoder {}

fn diva(a: u64, b: u64) -> u64 {
    if a % b == 0 { a / b } else { a / b + 1 }
}
