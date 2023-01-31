use self::ErrorType::*;
use crate::tiff::{jpeg_in_tiff, property, tag::*, Data, ParseTiffError, Parser, Tiff};
use std::{
    collections::HashMap,
    error, fmt,
    fs::File,
    io,
    io::{Read, Seek, SeekFrom},
};

#[derive(Debug)]
pub struct EozinError {
    t: ErrorType,
}

#[derive(Debug)]
enum ErrorType {
    IoError(io::Error),
    TiffError(ParseTiffError),
    ParseWsiError(String),
    MiscError(String),
}
impl fmt::Display for EozinError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.t {
            IoError(e) => write!(f, "IO Error {}", e).unwrap(),
            TiffError(e) => write!(f, "Parse Tiff Error {}", e).unwrap(),
            ParseWsiError(e) => write!(
                f,
                "Couldn't interpret given tiff file as Whole Slide Image {}",
                e
            )
            .unwrap(),
            MiscError(e) => write!(f, "Error {}", e).unwrap(),
        }
        Ok(())
    }
}

impl From<io::Error> for EozinError {
    fn from(err: io::Error) -> EozinError {
        EozinError { t: IoError(err) }
    }
}
impl From<ParseTiffError> for EozinError {
    fn from(err: ParseTiffError) -> EozinError {
        EozinError { t: TiffError(err) }
    }
}
impl error::Error for EozinError {}

#[non_exhaustive]
pub enum Tile {
    Jpeg(Vec<u8>),
    Jp2k(Vec<u8>),
}

impl Tile {
    pub fn buffer(&self) -> &Vec<u8> {
        match self {
            Tile::Jpeg(v) => v,
            Tile::Jp2k(v) => v,
        }
    }
}

pub struct Eozin {
    format: Format,
    pub level_count: u64,
    pub dimensions: (u64, u64),
    pub level_dimensions: Vec<(u64, u64)>,
    pub level_tile_sizes: Vec<(u64, u64)>,
}

enum Format {
    FormatAperio(Aperio),
}

pub struct Aperio {
    data: Tiff,
    file: File,
    levels: Vec<AperioLevel>,
    pub level_count: u64,
    pub dimensions: (u64, u64),
    pub level_dimensions: Vec<(u64, u64)>,
    pub level_tile_sizes: Vec<(u64, u64)>,
}

struct AperioLevel {
    compression: u16,
    jpeg_tables: Option<Vec<u8>>,
    pub t: property::TiledIfd,
}

impl Eozin {
    pub fn open(path: &str) -> Result<Self, EozinError> {
        match Aperio::open(path) {
            Ok(aperio) => Ok(Eozin {
                level_count: aperio.level_count,
                dimensions: aperio.dimensions.clone(),
                level_dimensions: aperio.level_dimensions.clone(),
                level_tile_sizes: aperio.level_tile_sizes.clone(),
                format: Format::FormatAperio(aperio),
            }),
            Err(e) => Err(e),
        }
    }
    pub fn read_tile(&mut self, lv: usize, x: usize, y: usize) -> Result<Tile, EozinError> {
        match &mut self.format {
            Format::FormatAperio(ap) => ap.read_tile(lv, x, y),
        }
    }
}

impl Aperio {
    pub fn open(path: &str) -> Result<Self, EozinError> {
        let mut file = File::open(path)?;
        let data = decode_file(&mut file)?;
        let mut levels = Vec::new();
        let mut level_dimensions = Vec::new();
        let mut level_tile_sizes = Vec::new();
        let mut maybe_dimensions = None;
        for ifd in data.iter() {
            if ifd.contains_key(&TileOffsets) {
                let maybe_cmp = ifd.get(&Compression).and_then(expect_short);
                let jpeg_tables = ifd.get(&JPEGTables).and_then(u8vec).map(|v| v.clone());
                let jpeg_tables = match jpeg_tables {
                    Some(mut jptb) => {
                        jpeg_in_tiff::set_app14_as_unknown(&mut jptb);
                        Some(jptb)
                    }
                    None => None,
                };
                if let (Some(compression), Some(t)) = (maybe_cmp, property::tiled_ifd(ifd)) {
                    level_dimensions.push((t.width, t.height));
                    level_tile_sizes.push((t.tile_width, t.tile_height));
                    maybe_dimensions = maybe_dimensions.or(Some((t.width, t.height)));
                    let lv = AperioLevel {
                        compression,
                        jpeg_tables,
                        t,
                    };
                    levels.push(lv);
                }
            }
        }
        if let Some(dimensions) = maybe_dimensions {
            Ok(Aperio {
                data,
                file,
                levels,
                dimensions,
                level_count: level_dimensions.len() as u64,
                level_dimensions,
                level_tile_sizes,
            })
        } else {
            Err(EozinError {
                t: MiscError("Coundn't find any IFD on input fileh".to_string()),
            })
        }
    }

    pub fn read_tile(&mut self, lv: usize, x: usize, y: usize) -> Result<Tile, EozinError> {
        let lv = self.levels.get(lv).ok_or(missing("level"))?;
        let num_tiles_across = (lv.t.width + lv.t.tile_width - 1) / lv.t.tile_width;
        let tile_id = (num_tiles_across as usize) * y + x;
        let (addr, len) =
            lv.t.offsets
                .get(tile_id)
                .and_then(|a| lv.t.byte_counts.get(tile_id).and_then(|l| Some((*a, *l))))
                .ok_or(missing("selected tile is out of index"))?;
        let buf = read_bytes(&mut self.file, addr as u64, (addr + len) as u64)?;
        match (&lv.jpeg_tables, lv.compression) {
            (Some(j_tb), 7) => {
                let mut jpeg_tables: Vec<u8> = j_tb.clone();
                let _ = jpeg_tables.split_off(jpeg_tables.len() - 2);
                jpeg_tables.extend_from_slice(&buf[2..]);
                Ok(Tile::Jpeg(jpeg_tables))
            }
            (_, 33003) => {
                println!("JP2k YCbCr");
                Ok(Tile::Jp2k(buf))
            }
            (_, 33005) => {
                println!("JP2k RGB");
                Ok(Tile::Jp2k(buf))
            }
            _ => Err(EozinError {
                t: MiscError("Unknown compression".to_string()),
            }),
        }
    }
}

fn u8vec(d: &Data) -> Option<&Vec<u8>> {
    match d {
        Data::UndefinedVec(v) => Some(v),
        Data::ByteVec(v) => Some(v),
        _ => None,
    }
}

fn expect_short(d: &Data) -> Option<u16> {
    match d {
        Data::Short(b) => Some(*b),
        _ => None,
    }
}

fn decode_file(file: &mut File) -> Result<Tiff, EozinError> {
    let buf = read_bytes(file, 0, 16)?;
    let (p, ifd_offset) = Parser::header(&buf)?;
    let size = p.size();
    let mut next_ifd = Some(ifd_offset);
    let mut directories: Tiff = Vec::new();
    while let Some(ofs) = next_ifd {
        let mut entries = HashMap::new();
        let mut unloaded = Vec::new();
        let buf = read_bytes(file, ofs, ofs + size.ifd_header)?;
        let count = p.ifd_count(&buf)?;
        let buf = read_bytes(
            file,
            ofs + size.ifd_header,
            ofs + size.ifd_header + size.ifd_body(count),
        )?;
        next_ifd = p.ifd_body(&buf, &mut entries, &mut unloaded)?;
        p.ifd_body(&buf, &mut entries, &mut unloaded)?;
        for (tag, c, dt, addr, len) in unloaded.into_iter() {
            let buf = read_bytes(file, addr, addr + len)?;
            let data = p.entry(c, dt, &buf)?;
            entries.insert(tag, data);
        }
        directories.push(entries);
    }
    Ok(directories)
}

fn read_bytes(file: &mut File, start: u64, end: u64) -> Result<Vec<u8>, EozinError> {
    let len = (end - start) as usize;
    let mut buffer = vec![0; len];
    file.seek(SeekFrom::Start(start))?;
    let l = file.read(&mut buffer)?;
    if l == len {
        Ok(buffer)
    } else {
        Err(EozinError {
            t: MiscError("Buffer length is not match".to_string()),
        })
    }
}

fn missing(s: &str) -> EozinError {
    EozinError {
        t: ParseWsiError(s.to_string()),
    }
}
