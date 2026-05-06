use crate::base::{
    DecoderConstructor, EozinDecoderCore, ImageType, LevelInfo, LevelInfoHandler, MapOutput,
    OutputMapper, ReadCommand, ReadConsumer, ReadTileDecoder,
};
use crate::error::EozinError::{self, *};
use crate::tiff_tools::{CommonTag::*, Ifd, TiffDecoder};

pub(crate) type AperioDecoderConstructor = MapOutput<TiffDecoder, IfdToDecoder>;

impl DecoderConstructor for AperioDecoderConstructor {}

pub(crate) type AperioReadTile = ReadTile;
pub(crate) type AperioReadTileInput = ReadTileInput;

pub(crate) struct IfdToDecoder {}

impl OutputMapper for IfdToDecoder {
    type Input = Vec<Ifd>;
    type Output = AperioDecoder;
    type ErrorKind = EozinError;
    fn map(input: Self::Input) -> Result<Self::Output, Self::ErrorKind> {
        AperioDecoder::with_ifds(&input)
    }
}

#[derive(Debug)]
pub(crate) struct AperioDecoder {
    levels: Vec<AperioLevel>,
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum AperioLevel {
    TiledJpegLevel(TiledJpeg),
    TiledJp2kLevel(TiledJp2k),
    // TODO: Add variants for associated images(macro, label, thumbnail).
}

#[derive(Debug)]
struct TiledJpeg {
    info: LevelInfo,
    tile_offsets: Vec<u64>,
    tile_byte_counts: Vec<u64>,
    jpeg_tables: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Debug)]
struct TiledJp2k {
    info: LevelInfo,
    is_rgb: bool, // Otherwise, YCbCr
    tile_offsets: Vec<u64>,
    tile_byte_counts: Vec<u64>,
}

impl AperioDecoder {
    pub fn with_ifds(dirs: &[Ifd]) -> Result<AperioDecoder, EozinError> {
        let mut levels = Vec::new();
        for ifd in dirs {
            match AperioLevel::from_ifd(ifd) {
                Some(lv) => levels.push(lv),
                _ => continue,
            }
        }
        Ok(AperioDecoder { levels })
    }
}

impl LevelInfoHandler for AperioDecoder {
    fn get_level(&self, i: usize) -> Option<LevelInfo> {
        self.levels.get(i).map(|l| l.info())
    }
    fn level_count(&self) -> usize {
        self.levels.len()
    }
    fn marginal_tile_size(&self, _lv: usize) -> Option<(u64, u64)> {
        None
    }
}

impl ReadTileDecoder for AperioDecoder {
    type ReadTile = ReadTile;
    type ReadTileInput = ReadTileInput;
    fn read_tile(&self, lv: usize, x: usize, y: usize) -> Result<ReadTileInput, EozinError> {
        use AperioLevel::*;
        use ReadTileInput::*;
        let lv = self.levels.get(lv).ok_or(LevelNotFound {
            selected: lv,
            num_level: self.levels.len(),
        })?;
        match lv {
            TiledJp2kLevel(t) => {
                let tile_id = t.info.tile_range_x * y + x;
                let addr = t.tile_offsets.get(tile_id);
                let count = t.tile_byte_counts.get(tile_id);
                let (&addr, &count) = addr.zip(count).ok_or(TileNotFound { x, y, z: 0 })?;
                Ok(ReadTileInputJp2k {
                    offset: addr,
                    len: count,
                })
            }
            TiledJpegLevel(t) => {
                let tile_id = t.info.tile_range_x * y + x;
                let addr = t.tile_offsets.get(tile_id);
                let count = t.tile_byte_counts.get(tile_id);
                let (&addr, &count) = addr.zip(count).ok_or(TileNotFound { x, y, z: 0 })?;
                Ok(ReadTileInputJpeg {
                    jpeg_tables: t.jpeg_tables.clone(),
                    offset: addr,
                    len: count,
                })
            }
        }
    }
}

impl EozinDecoderCore for AperioDecoder {}

#[derive(Debug)]
pub(crate) enum ReadTileInput {
    ReadTileInputJpeg {
        jpeg_tables: Vec<u8>,
        offset: u64,
        len: u64,
    },
    ReadTileInputJp2k {
        offset: u64,
        len: u64,
    },
}

impl Default for ReadTileInput {
    fn default() -> Self {
        ReadTileInput::ReadTileInputJp2k { offset: 0, len: 0 }
    }
}

#[derive(Debug)]
pub(crate) enum ReadTile {
    ReadTileJpeg(Vec<u8>),
    ReadTileJp2k,
}

impl ReadConsumer for ReadTile {
    type Input = ReadTileInput;
    type Output = Vec<u8>;
    type ErrorKind = EozinError;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand) {
        use ReadCommand::*;
        use ReadTile::*;
        use ReadTileInput::*;
        match input {
            ReadTileInputJpeg {
                jpeg_tables,
                offset,
                len: count,
            } => (ReadTileJpeg(jpeg_tables), ReadBytes { offset, count }),
            ReadTileInputJp2k { offset, len: count } => (ReadTileJp2k, ReadBytes { offset, count }),
        }
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        use ReadTile::*;
        match self {
            ReadTileJpeg(mut jpeg_tables) => {
                let _ = jpeg_tables.split_off(jpeg_tables.len() - 2);
                jpeg_tables.extend_from_slice(&buf[2..]);
                Ok(jpeg_tables)
            }
            ReadTileJp2k => Ok(buf.to_vec()),
        }
    }
    fn step(&mut self, _buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        Err(UnexpectedStep)
    }
}

impl AperioLevel {
    fn info(&self) -> LevelInfo {
        use self::AperioLevel::*;
        match self {
            TiledJpegLevel(l) => l.info,
            TiledJp2kLevel(l) => l.info,
        }
    }

    fn from_ifd(ifd: &Ifd) -> Option<Self> {
        use AperioLevel::*;
        let width = ifd.get(ImageWidth).and_then(|d| d.to_u64())?;
        let height = ifd.get(ImageLength).and_then(|d| d.to_u64())?;
        let tile_width = ifd.get(TileWidth).and_then(|d| d.to_u64())?;
        let tile_height = ifd.get(TileLength).and_then(|d| d.to_u64())?;
        let tile_offsets = ifd.get(TileOffsets).and_then(|d| d.to_u64vec())?;
        let tile_byte_counts = ifd.get(TileByteCounts).and_then(|d| d.to_u64vec())?;
        let compression = ifd.get(Compression).and_then(|d| d.to_u64())?;

        if compression == 7 {
            let mut jpeg_tables = ifd.get(JPEGTables).and_then(|d| d.expect_u8vec())?;
            if ifd.get(PhotometricInterpretation).and_then(|d| d.to_u64()) == Some(2) {
                // Jpeg and RGB
                set_app14_as_unknown(&mut jpeg_tables);
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
            let tiled_jp = TiledJpeg {
                info,
                tile_offsets,
                tile_byte_counts,
                jpeg_tables,
            };
            Some(TiledJpegLevel(tiled_jp))
        } else if compression == 33005 || compression == 33003 {
            let is_rgb = compression == 33005;
            let info = LevelInfo {
                width,
                height,
                tile_width,
                tile_height,
                tile_range_x: diva(width, tile_width) as usize,
                tile_range_y: diva(height, tile_height) as usize,
                image_type: if is_rgb {
                    ImageType::Jp2kRgb
                } else {
                    ImageType::Jp2kYCbCr
                },
            };
            let tiled_jp2k = TiledJp2k {
                info,
                tile_offsets,
                tile_byte_counts,
                is_rgb,
            };
            Some(TiledJp2kLevel(tiled_jp2k))
        } else {
            None
        }
    }
}

fn diva(a: u64, b: u64) -> u64 {
    if a % b == 0 { a / b } else { a / b + 1 }
}

const APP14_SEGMENT_TRANSFORM_UNKNOWN: [u8; 16] = [
    0xff, 0xee, 0x00, 0x0e, 0x41, 0x64, 0x6f, 0x62, 0x65, 0x00, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00,
];

pub(crate) fn set_app14_as_unknown(jpeg_tables: &mut Vec<u8>) {
    let (mut app14_ofs, mut dht_ofs) = (None, None);
    for (ofs, window) in jpeg_tables.windows(2).enumerate() {
        if window == [0xff, 0xee] {
            app14_ofs = Some(ofs);
            break;
        } else if window == [0xff, 0xc4] {
            dht_ofs = Some(ofs);
        }
    }
    match (app14_ofs, dht_ofs) {
        (Some(i), _) => {
            jpeg_tables[i + 16] = 0x00;
        }
        (None, Some(sos_ofs)) => {
            jpeg_tables.splice(
                sos_ofs..sos_ofs,
                APP14_SEGMENT_TRANSFORM_UNKNOWN.iter().copied(),
            );
        }
        _ => {}
    }
}
