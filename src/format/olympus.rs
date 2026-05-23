use crate::base::{
    DecoderConstructor, EozinDecoderCore, ImageType, LevelInfo, LevelInfoHandler,
    ReadCommand::{self, *},
    ReadConsumer, ReadTileDecoder,
};
use crate::error::{
    DecodeError::*,
    EozinError::{self, *},
};
use std::collections::HashMap;

const MAX_TILE_COUNT: u64 = 1024 * 1024;

#[derive(Debug)]
pub(crate) enum EtsDecoderConstructor {
    Dispatched,
    ReqEtsHeader(SisHeader),
    ReqEtsTiles {
        sis_header: SisHeader,
        ets_header: EtsHeader,
    },
}
impl DecoderConstructor for EtsDecoderConstructor {}

#[derive(Debug)]
pub struct EtsDecoder {
    levels: Vec<EtsLevel>,
    header: EtsHeader,
    tile_width: u64,
    tile_height: u64,
}

impl EozinDecoderCore for EtsDecoder {}

#[derive(Debug)]
pub struct EtsLevel {
    tile_range_x: u64,
    tile_range_y: u64,
    tile_range_z: u64,
    width: u64,
    height: u64,
    tiles: Vec<Option<(u64, u64)>>,
}

#[derive(Debug)]
pub struct TmpLevel {
    x: u32,
    y: u32,
    z: u32,
}

impl ReadConsumer for EtsDecoderConstructor {
    type Input = ();
    type Output = EtsDecoder;
    type ErrorKind = EozinError;

    fn dispatch(_input: Self::Input) -> (Self, ReadCommand) {
        (
            EtsDecoderConstructor::Dispatched,
            ReadBytesStep {
                offset: 0,
                count: 64,
            },
        )
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        use EtsDecoderConstructor::*;
        if let ReqEtsTiles {
            sis_header,
            ets_header,
        } = self
        {
            EtsDecoder::from_buf(sis_header, ets_header, buf)
        } else {
            Err(OlympusDecodingError("Failed to parse ETS"))?
        }
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        use EtsDecoderConstructor::*;
        match self {
            Dispatched => {
                let sis_header = SisHeader::from_buf(buf)
                    .ok_or(OlympusDecodingError("Failed to parse SIS header"))?;
                let r = ReadBytesStep {
                    offset: sis_header.ets_offset,
                    count: sis_header.ets_count as u64,
                };
                *self = ReqEtsHeader(sis_header);
                Ok(r)
            }
            ReqEtsHeader(_) => {
                let ets_header = EtsHeader::from_buf(buf)
                    .ok_or(OlympusDecodingError("Failed to parse SIS header"))?;
                let mut _self = Dispatched;
                std::mem::swap(&mut _self, self);
                if let ReqEtsHeader(sis_header) = _self {
                    let t_chunk = 9 * 4;
                    let tile_buf_size = t_chunk * (sis_header.num_used_chunks as u64);
                    let r = ReadBytes {
                        offset: sis_header.used_chunk_offset,
                        count: tile_buf_size,
                    };
                    *self = ReqEtsTiles {
                        sis_header,
                        ets_header,
                    };
                    Ok(r)
                } else {
                    panic!();
                }
            }
            _ => Err(UnexpectedStep),
        }
    }
}

impl EtsDecoder {
    fn from_buf(
        _sis_header: SisHeader,
        ets_header: EtsHeader,
        buf: &[u8],
    ) -> Result<EtsDecoder, EozinError> {
        let t_chunk = 9 * 4;
        let mut tiles = Vec::new();
        let mut tmp_levels: HashMap<u32, TmpLevel> = HashMap::new();
        for b in buf.chunks_exact(t_chunk as usize) {
            if let Some(t) = EtsTile::from_buf(b) {
                if let Some(lv) = tmp_levels.get_mut(&t.level) {
                    lv.x = lv.x.max(t.x);
                    lv.y = lv.y.max(t.y);
                    lv.z = lv.z.max(t.z);
                } else {
                    tmp_levels.insert(
                        t.level,
                        TmpLevel {
                            x: t.x,
                            y: t.y,
                            z: t.z,
                        },
                    );
                }
                tiles.push(t);
            }
        }
        let mut levels: Vec<EtsLevel> = Vec::new();
        let max_lv = tmp_levels.keys().max().unwrap();
        let (tw, th) = (ets_header.dim_x, ets_header.dim_y);
        for l in 0..=*max_lv {
            let tmp = tmp_levels
                .get(&l)
                .ok_or(OlympusDecodingError("Failed to decode ETS"))?;
            let tile_range_x = (tmp.x + 1) as u64;
            let tile_range_y = (tmp.y + 1) as u64;
            let tile_range_z = (tmp.z + 1) as u64;
            let n = tile_range_z * tile_range_x * tile_range_y;
            if n > MAX_TILE_COUNT {
                return Err(UnableToAllocateLargeSize);
            }
            let lv = EtsLevel {
                tile_range_x,
                tile_range_y,
                tile_range_z,
                width: (tw * (1 + tmp.x)) as u64,
                height: (th * (1 + tmp.y)) as u64,
                tiles: vec![None; n as usize],
            };
            levels.push(lv);
        }
        for t in tiles {
            let lv = levels.get_mut(t.level as usize).unwrap();
            let i = (t.x as usize)
                + (t.y as usize) * (lv.tile_range_x as usize)
                + (lv.tile_range_x as usize * lv.tile_range_y as usize) * (t.z as usize);
            lv.tiles[i] = Some((t.offset, t.count as u64));
        }
        Ok(EtsDecoder {
            levels,
            header: ets_header.clone(),
            tile_width: tw as u64,
            tile_height: th as u64,
        })
    }

    fn read_tile_with_z(
        &self,
        lv: usize,
        x: usize,
        y: usize,
        z: usize,
    ) -> Result<EtsReadTileInput, EozinError> {
        let lv = self.levels.get(lv).ok_or(LevelNotFound {
            selected: lv,
            num_level: self.levels.len(),
        })?;
        if x >= lv.tile_range_x as usize
            || y >= lv.tile_range_y as usize
            || z >= lv.tile_range_z as usize
        {
            return Err(TileNotFound { x, y, z })?;
        }
        let i = x
            + y * (lv.tile_range_x as usize)
            + (lv.tile_range_x as usize * lv.tile_range_y as usize) * z;
        if let Some(Some((offset, count))) = lv.tiles.get(i) {
            Ok(EtsReadTileInput {
                offset: *offset,
                count: *count,
            })
        } else {
            Err(TileNotFound { x, y, z })?
        }
    }
}

impl LevelInfoHandler for EtsDecoder {
    fn get_level(&self, i: usize) -> Option<LevelInfo> {
        let lv = self.levels.get(i)?;
        let image_type = match self.header.compression {
            EtsCompressionType::Jpeg => Some(ImageType::Jpeg),
            EtsCompressionType::Jpeg2k => Some(ImageType::Jp2kRgb),
            EtsCompressionType::Bmp => Some(ImageType::Bmp),
            _ => None,
        }?;
        Some(LevelInfo {
            width: lv.width,
            height: lv.height,
            tile_width: self.tile_width,
            tile_height: self.tile_height,
            tile_range_x: lv.tile_range_x as usize,
            tile_range_y: lv.tile_range_y as usize,
            image_type
        })
    }
    fn level_count(&self) -> usize {
        self.levels.len()
    }
    fn marginal_tile_size(&self, _lv: usize) -> Option<(u64, u64)> {
        None
    }
}

impl ReadTileDecoder for EtsDecoder {
    type ReadTile = EtsReadTile;
    type ReadTileInput = EtsReadTileInput;
    fn read_tile(&self, lv: usize, x: usize, y: usize) -> Result<EtsReadTileInput, EozinError> {
        self.read_tile_with_z(lv, x, y, 0)
    }
}

#[derive(Debug, Default)]
pub(crate) struct EtsReadTileInput {
    pub offset: u64,
    pub count: u64,
}

#[derive(Debug)]
pub(crate) struct EtsReadTile {}

impl ReadConsumer for EtsReadTile {
    type Input = EtsReadTileInput;
    type Output = Vec<u8>;
    type ErrorKind = EozinError;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand) {
        let EtsReadTileInput { offset, count } = input;
        let r = ReadBytes { offset, count };
        (EtsReadTile {}, r)
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        Ok(buf.to_vec())
    }
    fn step(&mut self, _buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        Err(UnexpectedStep)
    }
}

// Olympus SIS Header
#[allow(dead_code)]
#[derive(Debug)]
pub struct SisHeader {
    pub version: u32,
    pub dim: u32,
    pub ets_offset: u64,
    pub ets_count: u32,
    pub used_chunk_offset: u64,
    pub num_used_chunks: u32,
}

impl SisHeader {
    fn from_buf(buf: &[u8]) -> Option<SisHeader> {
        let mut r = BytesParser::new(buf);
        let _ = r
            .consume_ascii(4)
            .and_then(|s| if s == "SIS\0" { Some(true) } else { None })?;
        let _header_size = r.consume_u32()?;
        let version = r.consume_u32()?;
        let dim = r.consume_u32()?;
        let ets_offset = r.consume_u64()?;
        let ets_count = r.consume_u32()?;
        let _ = r.consume(4)?; // Skip bytes(4), Reserved Property.
        let used_chunk_offset = r.consume_u64()?;
        let num_used_chunks = r.consume_u32()?;
        let header = SisHeader {
            version,
            dim,
            ets_offset,
            ets_count,
            used_chunk_offset,
            num_used_chunks,
        };
        Some(header)
    }
}

// Olympus ETS Header
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct EtsHeader {
    pub version: u32,
    pub pixel_type: EtsPixelType,
    pub num_color_channel: u32,
    pub color_space: EtsColorSpace,
    pub compression: EtsCompressionType,
    pub quality: u32,
    pub dim_x: u32,
    pub dim_y: u32,
    pub dim_z: u32,
    pub background_color: [u8; 3],
    pub use_pyramid: bool,
}

impl EtsHeader {
    fn from_buf(buf: &[u8]) -> Option<EtsHeader> {
        let mut r = BytesParser::new(buf);
        let _ = r
            .consume_ascii(4)
            .and_then(|s| if s == "ETS\0" { Some(true) } else { None })?;
        let version = r.consume_u32()?;
        let pixel_type = r.consume_u32().and_then(EtsPixelType::from_u32)?;
        let num_color_channel = r.consume_u32()?;
        let color_space = r.consume_u32().and_then(EtsColorSpace::from_u32)?;
        let compression = r.consume_u32().and_then(EtsCompressionType::from_u32)?;
        let quality = r.consume_u32()?;
        let dim_x = r.consume_u32()?;
        let dim_y = r.consume_u32()?;
        let dim_z = r.consume_u32()?;
        let _ = r.consume(4 * 17);
        let (background_color, _l) = match pixel_type {
            EtsPixelType::Char | EtsPixelType::Uchar => ([0, 0, 0], 4 * 10 - 3),
            _ => ([0, 0, 0], 4 * 10 - 3), // TODO: implement later
        };
        let _ = r.consume(4 * 10);
        let _component_order = r.consume_u32()?;
        let use_pyramid = r.consume_u32().map(|i| i != 0)?;
        let header = EtsHeader {
            version,
            pixel_type,
            num_color_channel,
            color_space,
            compression,
            quality,
            dim_x,
            dim_y,
            dim_z,
            background_color,
            use_pyramid,
        };
        Some(header)
    }
}

// Olympus ETS Tile
#[derive(Debug)]
pub struct EtsTile {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub level: u32,
    pub offset: u64,
    pub count: u32,
}

impl EtsTile {
    fn from_buf(buf: &[u8]) -> Option<EtsTile> {
        let mut r = BytesParser::new(buf);
        let _ = r.consume(4)?;
        let x = r.consume_u32()?;
        let y = r.consume_u32()?;
        let z = r.consume_u32()?;
        let level = r.consume_u32()?;
        let offset = r.consume_u64()?;
        let count = r.consume_u32()?;
        let tile = EtsTile {
            x,
            y,
            z,
            level,
            offset,
            count,
        };
        Some(tile)
    }
}

#[derive(Debug, Clone)]
pub enum EtsPixelType {
    Char,   // 1 i8
    Uchar,  // 2 u8
    Short,  // 3 i16
    Ushort, // 4 u16
}

#[derive(Debug, Clone)]
pub enum EtsColorSpace {
    Fluorescence, // 1
    Brightfield,  // 4
}

#[derive(Debug, Clone)]
pub enum EtsCompressionType {
    Raw,          // 0
    Jpeg,         // 2
    Jpeg2k,       // 3
    JpegLossless, // 5
    Png,          // 8
    Bmp,          // 9
}
#[allow(dead_code)]
impl EtsCompressionType {
    pub fn from_u32(n: u32) -> Option<Self> {
        use EtsCompressionType::*;
        match n {
            0 => Some(Raw),
            2 => Some(Jpeg),
            3 => Some(Jpeg2k),
            5 => Some(JpegLossless),
            8 => Some(Png),
            9 => Some(Bmp),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl EtsPixelType {
    pub fn from_u32(n: u32) -> Option<Self> {
        use EtsPixelType::*;
        match n {
            1 => Some(Char),
            2 => Some(Uchar),
            3 => Some(Short),
            4 => Some(Ushort),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl EtsColorSpace {
    pub fn from_u32(n: u32) -> Option<Self> {
        use EtsColorSpace::*;
        match n {
            1 => Some(Fluorescence),
            4 => Some(Brightfield),
            _ => None,
        }
    }
}

struct BytesParser<'a> {
    buf: &'a [u8],
    cur: usize,
}

#[allow(dead_code)]
impl<'a> BytesParser<'a> {
    fn new(buf: &'a [u8]) -> BytesParser<'a> {
        BytesParser { buf, cur: 0 }
    }
    fn consume_u8(&mut self) -> Option<u8> {
        if self.cur < self.buf.len() {
            let r = self.buf[self.cur];
            self.cur += 1;
            Some(r)
        } else {
            None
        }
    }
    fn consume_u32(&mut self) -> Option<u32> {
        self.consume(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn consume_u64(&mut self) -> Option<u64> {
        self.consume(8)
            .map(|b| u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }
    fn consume_ascii(&mut self, n: usize) -> Option<String> {
        self.consume(n).map(|b| Self::ascii(&b))
    }
    fn consume(&mut self, n: usize) -> Option<Vec<u8>> {
        if self.cur + n < self.buf.len() {
            let r = self.buf[self.cur..self.cur + n].to_vec();
            self.cur += n;
            Some(r)
        } else {
            None
        }
    }
    fn ascii(buf: &[u8]) -> String {
        buf.iter().fold(String::new(), |mut acc, x| {
            if let Some(c) = std::char::from_u32(*x as u32) {
                acc.push(c);
            }
            acc
        })
    }
}
