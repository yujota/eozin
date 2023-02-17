use crate::tiff::{jpeg_in_tiff, property, tag::*, Data, ParseTiffError, Parser, Tiff};
use js_sys::{Array, Uint8ClampedArray};
use std::collections::HashMap;
use std::error;
use std::fmt;
use wasm_bindgen::prelude::*;

struct AperioLevel {
    compression: u16,
    jpeg_tables: Option<Vec<u8>>,
    pub t: property::TiledIfd,
}
pub struct Aperio {
    data: Tiff,
    blob: web_sys::Blob,
    levels: Vec<AperioLevel>,
    pub level_count: u64,
    pub dimensions: (u64, u64),
    pub level_dimensions: Vec<(u64, u64)>,
    pub level_tile_sizes: Vec<(u64, u64)>,
}

impl Aperio {
    pub async fn open(blob: web_sys::Blob) -> Result<Self, EozinError> {
        let data = decode_blob(&blob).await?;
        let level_count: u64 = data.iter().fold(0, |acc, ifd| {
            acc + property::tiled_ifd(ifd).map_or(0, |_| 1)
        });
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
                blob,
                levels,
                dimensions,
                level_count: level_dimensions.len() as u64,
                level_dimensions,
                level_tile_sizes,
            })
        } else {
            Err(EozinError {
                msg: "Coundn't find any IFD on input fileh".to_string(),
            })
        }
    }

    pub async fn read_tile(
        &mut self,
        lv: usize,
        x: usize,
        y: usize,
    ) -> Result<web_sys::Blob, EozinError> {
        let lv = self.levels.get(lv).ok_or(missing("level"))?;
        let num_tiles_across = (lv.t.width + lv.t.tile_width - 1) / lv.t.tile_width;
        let tile_id = (num_tiles_across as usize) * y + x;
        let (addr, len) =
            lv.t.offsets
                .get(tile_id)
                .and_then(|a| lv.t.byte_counts.get(tile_id).and_then(|l| Some((*a, *l))))
                .ok_or(missing("selected tile is out of index"))?;
        let buf = read_bytes(&self.blob, addr, addr + len).await?;
        match (&lv.jpeg_tables, lv.compression) {
            (Some(j_tb), 7) => {
                let mut jpeg_tables: Vec<u8> = j_tb.clone();
                let _ = jpeg_tables.split_off(jpeg_tables.len() - 2);
                jpeg_tables.extend_from_slice(&buf[2..]);
                let array = unsafe { Uint8ClampedArray::view(&jpeg_tables) };
                let blob_array = Array::new();
                blob_array.set(0, array.into());
                let mut options = web_sys::BlobPropertyBag::new();
                options.type_("image/jpeg");
                web_sys::Blob::new_with_u8_array_sequence_and_options(&blob_array, &options)
                    .map_err(|e| e.into())
            }
            _ => Err(missing("Unknown compression")),
        }
    }
}

pub async fn level_count(blob: web_sys::Blob) -> Result<u64, EozinError> {
    let tiff = decode_blob(&blob).await?;
    let lv: u64 = tiff.iter().fold(0, |acc, ifd| {
        acc + property::tiled_ifd(ifd).map_or(0, |_| 1)
    });
    Ok(lv)
}

#[derive(Debug)]
pub struct EozinError {
    msg: String,
}

impl fmt::Display for EozinError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg).unwrap();
        Ok(())
    }
}

impl From<JsValue> for EozinError {
    fn from(err: JsValue) -> EozinError {
        EozinError {
            msg: format!("io error {:?}", err),
        }
    }
}

impl From<ParseTiffError> for EozinError {
    fn from(err: ParseTiffError) -> EozinError {
        EozinError {
            msg: format!("io error {:?}", err),
        }
    }
}

impl error::Error for EozinError {}

async fn decode_blob(blob: &web_sys::Blob) -> Result<Tiff, EozinError> {
    let buf = read_bytes(blob, 0, 16).await?;
    let (p, ifd_offset) = Parser::header(&buf)?;
    let size = p.size();
    let mut next_ifd = Some(ifd_offset);
    let mut directories: Tiff = Vec::new();
    while let Some(ofs) = next_ifd {
        let mut entries = HashMap::new();
        let mut unloaded = Vec::new();
        let buf = read_bytes(blob, ofs, ofs + size.ifd_header).await?;
        let count = p.ifd_count(&buf)?;
        let buf = read_bytes(
            blob,
            ofs + size.ifd_header,
            ofs + size.ifd_header + size.ifd_body(count),
        )
        .await?;
        next_ifd = p.ifd_body(&buf, &mut entries, &mut unloaded)?;
        p.ifd_body(&buf, &mut entries, &mut unloaded)?;
        for (tag, c, dt, addr, len) in unloaded.into_iter() {
            let buf = read_bytes(blob, addr, addr + len).await?;
            let data = p.entry(c, dt, &buf)?;
            entries.insert(tag, data);
        }
        directories.push(entries);
    }
    Ok(directories)
}

async fn read_bytes(blob: &web_sys::Blob, start: u64, end: u64) -> Result<Vec<u8>, EozinError> {
    let sliced_blob = blob.slice_with_f64_and_f64_and_content_type(
        start as f64,
        end as f64,
        "application/octet-stream",
    )?;
    let buffer = wasm_bindgen_futures::JsFuture::from(sliced_blob.array_buffer()).await?;
    let array = js_sys::Uint8Array::new(&buffer);
    Ok(array.to_vec())
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
fn missing(s: &str) -> EozinError {
    EozinError {
        msg: format!("missing {}", s),
    }
}
