use self::TmpError::HogeError;
use crate::tiff::{jpeg_in_tiff, property, tag::*, Data, ParseTiffError, Parser, Tiff};
use std::collections::HashMap;
use std::error;
use std::fmt;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

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
pub struct Aperio {
    data: Tiff,
    file: File,
    levels: Vec<AperioLevel>,
    pub level_count: u64,
    pub dimensions: (u64, u64),
    pub level_dimensions: Vec<(u64, u64)>,
}

struct AperioLevel {
    compression: u16,
    jpeg_tables: Option<Vec<u8>>,
    pub t: property::TiledIfd,
}

impl Aperio {
    pub fn open(path: &str) -> Result<Self, Box<dyn error::Error>> {
        let mut file = File::open(path)?;
        let data = decode_file(&mut file)?;
        let mut levels = Vec::new();
        let mut level_dimensions = Vec::new();
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
            })
        } else {
            Err(Box::new(HogeError("no ifd".to_string())))
        }
    }

    pub fn read_tile(
        &mut self,
        lv: usize,
        x: usize,
        y: usize,
    ) -> Result<Tile, Box<dyn error::Error>> {
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
            _ => Err(Box::new(HogeError("hoge".to_string()))),
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

fn expect_short(d: &Data) -> Option<u16> {
    match d {
        Data::Short(b) => Some(*b),
        _ => None,
    }
}
/*
pub fn read(path: &str) {
    let mut file = File::open(path).unwrap();
    match decode_file(file) {
        Ok(tiff) => println!("{:?}", tiff),
        Err(e) => println!("Error {:?}", e),
    }
}
*/
fn decode_file(file: &mut File) -> Result<Tiff, Box<dyn error::Error>> {
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
        println!("next_ifd:: {:?}", &next_ifd);
    }
    println!("parser:: {:?}", &p);
    Ok(directories)
}

fn read_bytes(file: &mut File, start: u64, end: u64) -> Result<Vec<u8>, Box<dyn error::Error>> {
    let len = (end - start) as usize;
    let mut buffer = vec![0; len];
    file.seek(SeekFrom::Start(start))?;
    let l = file.read(&mut buffer)?;
    if l == len {
        Ok(buffer)
    } else {
        Err(Box::new(TmpError::HogeError("hoge".to_string())))
    }
}

fn missing(s: &str) -> TmpError {
    TmpError::MissingError(s.to_string())
}
#[derive(Debug)]
enum TmpError {
    HogeError(String),
    MissingError(String),
}

impl fmt::Display for TmpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TmpError::HogeError(s) => write!(f, "hogeerror: {}", s),
            TmpError::MissingError(s) => write!(f, "missing: {}", s),
        }
    }
}

impl error::Error for TmpError {}
