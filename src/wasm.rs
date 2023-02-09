use crate::tiff::{jpeg_in_tiff, property, tag::*, Data, ParseTiffError, Parser, Tiff};
use std::collections::HashMap;
use std::error;
use std::fmt;
use wasm_bindgen::prelude::*;

pub struct Aperio {
    data: Tiff,
    blob: web_sys::Blob,
    pub level_count: u64,
}

impl Aperio {
    pub fn open(blob: web_sys::Blob) -> Self {
        todo!()
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
