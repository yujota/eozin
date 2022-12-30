use std::collections::HashMap;

use crate::tiff::error::{format_error, TiffError};
use crate::tiff::types::{
    BytesParser, BytesParser::*, Data, Data::*, DataOffset, DataType, DataType::*, Entry, Entry::*,
    TiffParser, TiffParser::*,
};

type Tag = u16;
type Count = u64;
type Offset = u64;
type IfdBodyStart = u64;
type IfdBodyEnd = u64;
type NextIfdOffsetStart = u64;
type NextIfdOffsetEnd = u64;

#[allow(dead_code)]
impl TiffParser {
    pub(in crate::tiff) fn num_entry(&self, i: &[u8]) -> Result<u64, TiffError> {
        match *self {
            Classic(p) => Ok(p.u16(i).unwrap() as u64),
            Big(p) => Ok(p.u64(i).unwrap()),
        }
    }

    pub(in crate::tiff) fn next_ifd(&self, i: &[u8]) -> Result<Option<u64>, TiffError> {
        match *self {
            Classic(p) => {
                if i.len() < 4 {
                    None
                } else {
                    p.u32(i).map(|x| x as u64)
                }
            }
            Big(p) => {
                if i.len() < 8 {
                    None
                } else {
                    p.u64(i)
                }
            }
        }
        .ok_or(format_error("Failed to parse next_ifd"))
        .map(|x| if x == 0 { None } else { Some(x) })
    }

    pub(in crate::tiff) fn ifd_header(
        &self,
        current_offset: u64,
        i: &[u8],
    ) -> Result<
        (
            Count,
            (IfdBodyStart, IfdBodyEnd),
            (NextIfdOffsetStart, NextIfdOffsetEnd),
        ),
        TiffError,
    > {
        let co = current_offset;
        match *self {
            Classic(p) => {
                if i.len() < 2 {
                    Err(format_error("Insufficiant input"))
                } else {
                    let count = p
                        .u16(&i[0..2])
                        .map(|x| x as u64)
                        .ok_or(format_error("Insufficiant input"))?;
                    println!("======== count {:?} ===", count);
                    let body_ofs = co + 2;
                    let next_ifd = body_ofs + 12 * count;
                    Ok((count, (body_ofs, next_ifd), (next_ifd, next_ifd + 4)))
                }
            }
            Big(p) => {
                if i.len() < 8 {
                    Err(format_error("Insufficiant input"))
                } else {
                    let count = p.u64(&i[0..8]).ok_or(format_error("Insufficiant input"))?;
                    let body_ofs = co + 8;
                    let next_ifd = body_ofs + 20 * count;
                    Ok((count, (body_ofs, next_ifd), (next_ifd, next_ifd + 8)))
                }
            }
        }
    }

    pub(in crate::tiff) fn ifd_body(
        &self,
        i: &[u8],
    ) -> (HashMap<Tag, Data>, Vec<(Tag, DataOffset)>, Vec<TiffError>) {
        let mut entries = HashMap::new();
        let mut offsets = Vec::new();
        let mut errors = Vec::new();
        let num_chunks = match *self {
            Classic(_) => 12,
            Big(_) => 20,
        };
        for buf in i.chunks(12) {
            match self.entry(buf) {
                Ok((t, DataEntry(d))) => {
                    entries.insert(t, d);
                }
                Ok((t, OffsetEntry(ofs))) => {
                    offsets.push((t, ofs));
                }
                Err(tiff_error) => {
                    errors.push(tiff_error);
                }
            }
        }
        (entries, offsets, errors)
    }

    pub(in crate::tiff) fn entry(&self, i: &[u8]) -> Result<(Tag, Entry), TiffError> {
        match *self {
            Classic(p) => {
                let tag = p.u16(&i[..2]).unwrap();
                let data_type = p
                    .u16(&i[2..4])
                    .and_then(DataType::from_u16)
                    .ok_or(format_error("Unknown data type"))?;
                let count = p.u32(&i[4..8]).unwrap() as u64;
                if data_type.size() * count <= 4 {
                    self.entry_data(count, data_type, &i[8..12])
                        .map(move |d| (tag, DataEntry(d)))
                } else {
                    let offset = p.u32(&i[8..12]).unwrap() as u64;
                    Ok((
                        tag,
                        OffsetEntry(DataOffset {
                            data_type,
                            count,
                            offset,
                        }),
                    ))
                }
            }
            Big(p) => {
                let tag = p.u16(&i[..2]).unwrap();
                let data_type = p
                    .u16(&i[2..4])
                    .and_then(DataType::from_u16)
                    .ok_or(format_error("Unknown data type"))?;
                let count = p.u64(&i[4..12]).unwrap();
                if data_type.size() * count <= 8 {
                    self.entry_data(count, data_type, &i[12..20])
                        .map(move |d| (tag, DataEntry(d)))
                } else {
                    let offset = p.u64(&i[12..20]).unwrap();
                    Ok((
                        tag,
                        OffsetEntry(DataOffset {
                            data_type,
                            count,
                            offset,
                        }),
                    ))
                }
            }
        }
    }

    pub(in crate::tiff) fn entry_data(
        &self,
        count: u64,
        data_type: DataType,
        i: &[u8],
    ) -> Result<Data, TiffError> {
        let dt_debug = data_type.clone();
        if data_type.size() * count > i.len() as u64 {
            return Err(format_error("Buffer is not enough"));
        }
        let p = match *self {
            Classic(p) => p,
            Big(p) => p,
        };
        match (data_type, count) {
            (BYTE, 1) => p.u8(i).map(Byte),
            (BYTE, n) => p.u8_vec(n, i).map(ByteVec),
            (UNDEFINED, 1) => p.u8(i).map(Undefined),
            (UNDEFINED, n) => p.u8_vec(n, i).map(UndefinedVec),
            (SHORT, 1) => p.u16(i).map(Short),
            (SHORT, n) => p.u16_vec(n, i).map(ShortVec),
            (LONG, 1) => p.u32(i).map(Long),
            (LONG, n) => p.u32_vec(n, i).map(LongVec),
            (LONG8, 1) => p.u64(i).map(Long8),
            (LONG8, n) => p.u64_vec(n, i).map(Long8Vec),
            (IFD8, 1) => p.u64(i).map(Ifd8),
            (IFD8, n) => p.u64_vec(n, i).map(Ifd8Vec),
            (ASCII, n) => p.ascii(n, i).map(Ascii),
            (RATIONAL, 1) => p.rational(i).map(|(n, d)| Rational { numer: n, denom: d }),
            (RATIONAL, c) => p.rational_vec(c, i).map(RationalVec),
            _ => None,
        }
        .ok_or(format_error(&format!(
            "DataType is not supported {:?}",
            dt_debug
        )))
    }
}

#[allow(dead_code)]
impl BytesParser {
    pub(in crate::tiff) fn ascii(&self, n: u64, i: &[u8]) -> Option<String> {
        let n = n as usize;
        if i.len() < n {
            return None;
        } else {
            Some(i[0..n].iter().fold(String::new(), |mut acc, x| {
                if let Some(c) = std::char::from_u32(*x as u32) {
                    acc.push(c);
                }
                acc
            }))
        }
    }

    pub(in crate::tiff) fn rational(&self, i: &[u8]) -> Option<(u32, u32)> {
        if i.len() >= 8 {
            match (self.u32(&i[0..4]), self.u32(&i[4..8])) {
                (Some(numer), Some(denom)) => Some((numer, denom)),
                _ => None,
            }
        } else {
            None
        }
    }

    pub(in crate::tiff) fn rational_vec(&self, n: u64, i: &[u8]) -> Option<Vec<(u32, u32)>> {
        let n = n as usize;
        if i.len() < n * 2 {
            return None;
        }
        Some(i[0..8 * n].chunks(8).fold(Vec::new(), |mut acc, x| {
            acc.push(self.rational(x).unwrap());
            acc
        }))
    }

    pub(in crate::tiff) fn u8(&self, i: &[u8]) -> Option<u8> {
        if i.len() < 1 {
            None
        } else {
            Some(i[0])
        }
    }

    pub(in crate::tiff) fn u8_vec(&self, n: u64, i: &[u8]) -> Option<Vec<u8>> {
        let n = n as usize;
        if i.len() < n {
            return None;
        } else {
            Some(i[0..n].to_vec())
        }
    }

    pub(in crate::tiff) fn u16(&self, i: &[u8]) -> Option<u16> {
        if i.len() < 2 {
            return None;
        }
        match *self {
            Intel => Some(((i[1] as u16) << 8) + i[0] as u16),
            Moto => Some(((i[0] as u16) << 8) + i[1] as u16),
        }
    }

    pub(in crate::tiff) fn u16_vec(&self, n: u64, i: &[u8]) -> Option<Vec<u16>> {
        let n = n as usize;
        if i.len() < n * 2 {
            return None;
        }
        Some(i[0..2 * n].chunks(2).fold(Vec::new(), |mut acc, x| {
            acc.push(self.u16(x).unwrap());
            acc
        }))
    }

    pub(in crate::tiff) fn u32(&self, i: &[u8]) -> Option<u32> {
        if i.len() < 4 {
            return None;
        }
        match *self {
            Intel => Some(
                ((i[3] as u32) << 24) + ((i[2] as u32) << 16) + ((i[1] as u32) << 8) + i[0] as u32,
            ),
            Moto => Some(
                ((i[0] as u32) << 24) + ((i[1] as u32) << 16) + ((i[2] as u32) << 8) + i[3] as u32,
            ),
        }
    }

    pub(in crate::tiff) fn u32_vec(&self, n: u64, i: &[u8]) -> Option<Vec<u32>> {
        let n = n as usize;
        if i.len() < n * 4 {
            return None;
        }
        Some(i[0..4 * n].chunks(4).fold(Vec::new(), |mut acc, x| {
            acc.push(self.u32(x).unwrap());
            acc
        }))
    }

    pub(in crate::tiff) fn u64(&self, i: &[u8]) -> Option<u64> {
        if i.len() < 8 {
            return None;
        }
        match *self {
            Intel => Some(
                ((i[7] as u64) << 56)
                    + ((i[6] as u64) << 48)
                    + ((i[5] as u64) << 40)
                    + ((i[4] as u64) << 32)
                    + ((i[3] as u64) << 24)
                    + ((i[2] as u64) << 16)
                    + ((i[1] as u64) << 8)
                    + i[0] as u64,
            ),
            Moto => Some(
                ((i[0] as u64) << 56)
                    + ((i[1] as u64) << 48)
                    + ((i[2] as u64) << 40)
                    + ((i[3] as u64) << 32)
                    + ((i[4] as u64) << 24)
                    + ((i[5] as u64) << 16)
                    + ((i[6] as u64) << 8)
                    + i[7] as u64,
            ),
        }
    }

    pub(in crate::tiff) fn u64_vec(&self, n: u64, i: &[u8]) -> Option<Vec<u64>> {
        let n = n as usize;
        if i.len() < n * 8 {
            return None;
        }
        Some(i[0..8 * n].chunks(8).fold(Vec::new(), |mut acc, x| {
            acc.push(self.u64(x).unwrap());
            acc
        }))
    }
}
