use self::{Bytes::*, ParseTiffError::*, Parser::*};
use super::data::{Data, Data::*, DataType, DataType::*};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

type Tag = u16;
type Count = u64;
type Offset = u64;

type Address = u64;
type Len = u64;

#[derive(Debug)]
pub(crate) enum ParseTiffError {
    TiffHeaderBroken(String),
    TiffEntryBroken(String),
    BufferLengthNotEnough(u64),
}

impl fmt::Display for ParseTiffError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TiffHeaderBroken(s) => write!(f, "Parsing tiff header is failed: {}", s).unwrap(),
            TiffEntryBroken(s) => write!(f, "Parsing tiff entry is failed: {}", s).unwrap(),
            BufferLengthNotEnough(s) => write!(
                f,
                "During parsing tiff provided buffer size is not enough need at least {}",
                s
            )
            .unwrap(),
        }
        Ok(())
    }
}
impl Error for ParseTiffError {}

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum Bytes {
    Intel,
    Moto,
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum Parser {
    Classic(Bytes),
    Big(Bytes),
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) struct Size {
    pub ifd_header: u64,
    ent_size: u64,
    next_ifd_ofs_size: u64,
}

impl Size {
    pub(crate) fn ifd_body(&self, c: u64) -> u64 {
        c * self.ent_size + self.next_ifd_ofs_size
    }
}
impl Parser {
    pub(crate) fn header(i: &[u8]) -> Result<(Self, Offset), ParseTiffError> {
        let bp = match Moto.u16(&i[0..2]).unwrap() {
            18761 => {
                // Little endian(Intel) 18761 == 0x49, 0x49
                Intel
            }
            19789 => {
                // Big endian(Motorola) 19789 == 0x4d, 0x4d
                Moto
            }
            dbg => return Err(TiffHeaderBroken(format!("Unknown endian: {:?}", dbg))),
        };
        match bp.u16(&i[2..4]).unwrap() {
            42 => {
                let next_ifd = bp.u32(&i[4..8]).unwrap() as u64;
                Ok((Classic(bp), next_ifd))
            }
            43 => {
                let i = &i[4..16];
                let (always_8, always_0) = (bp.u16(&i[..2]).unwrap(), bp.u16(&i[2..4]).unwrap());
                if always_8 == 8 && always_0 == 0 {
                    let next_ifd = bp.u64(&i[4..12]).unwrap();
                    Ok((Big(bp), next_ifd))
                } else {
                    Err(TiffHeaderBroken("Unknown tff version".to_string()))
                }
            }
            _ => Err(TiffHeaderBroken("Unknown tff version".to_string())),
        }
    }

    pub(crate) fn size(&self) -> Size {
        match *self {
            Classic(_) => Size {
                ifd_header: 2,
                next_ifd_ofs_size: 4,
                ent_size: 12,
            },
            Big(_) => Size {
                ifd_header: 8,
                next_ifd_ofs_size: 8,
                ent_size: 20,
            },
        }
    }

    pub(crate) fn ifd_count(&self, i: &[u8]) -> Result<u64, ParseTiffError> {
        match *self {
            Classic(p) => p.u16(i).map(|x| x as u64).ok_or(BufferLengthNotEnough(2)),

            Big(p) => p.u64(i).ok_or(BufferLengthNotEnough(8)),
        }
    }

    pub(crate) fn ifd_body(
        &self,
        i: &[u8],
        entries: &mut HashMap<Tag, Data>,
        unloaded: &mut Vec<(Tag, Count, DataType, Address, Len)>,
    ) -> Result<Option<Address>, ParseTiffError> {
        match *self {
            Classic(p) => {
                for j in i.chunks(12) {
                    let tag = p.u16(&j[..2]).unwrap();
                    if let Some(dt) = p.u16(&j[2..4]).and_then(DataType::from_u16) {
                        let count = p.u32(&j[4..8]).unwrap() as u64;
                        let len = dt.size() * count;
                        if len <= 4 {
                            let data = self.entry(count, dt, &j[8..12])?;
                            entries.insert(tag, data);
                        } else {
                            let offset = p.u32(&j[8..12]).unwrap() as u64;
                            unloaded.push((tag, count, dt, offset, len));
                        }
                    } else {
                        continue;
                    }
                }
                let idx = (i.len() / 12) * 12;
                Ok(p.u32(&i[idx..idx + 4])
                    .and_then(|x| if x == 0 { None } else { Some(x as u64) }))
            }
            Big(p) => {
                for j in i.chunks(20) {
                    let tag = p.u16(&j[..2]).unwrap();
                    if let Some(dt) = p.u16(&j[2..4]).and_then(DataType::from_u16) {
                        let count = p.u64(&j[4..12]).unwrap();
                        let len = dt.size() * count;
                        if len <= 8 {
                            let data = self.entry(count, dt, &j[12..20])?;
                            entries.insert(tag, data);
                        } else {
                            let offset = p.u64(&j[12..20]).unwrap();
                            unloaded.push((tag, count, dt, offset, len));
                        }
                    } else {
                        // Skipping undefined data_type
                        continue;
                    }
                }
                let idx = (i.len() / 20) * 20;
                Ok(p.u64(&i[idx..idx + 8])
                    .and_then(|x| if x == 0 { None } else { Some(x) }))
            }
        }
    }
    pub(crate) fn entry(&self, c: u64, dt: DataType, i: &[u8]) -> Result<Data, ParseTiffError> {
        let p = match *self {
            Classic(p) => p,
            Big(p) => p,
        };
        fn e(m: Option<Data>, s: &str) -> Result<Data, ParseTiffError> {
            m.ok_or(TiffEntryBroken(s.to_string()))
        }
        match (dt, c) {
            (BYTE, 1) => e(p.u8(i).map(Byte), "byte"),
            (BYTE, n) => e(p.u8_vec(n, i).map(ByteVec), "byte"),
            (UNDEFINED, 1) => e(p.u8(i).map(Undefined), "undefined"),
            (UNDEFINED, n) => e(p.u8_vec(n, i).map(UndefinedVec), "undefined"),
            (SHORT, 1) => e(p.u16(i).map(Short), "short"),
            (SHORT, n) => e(p.u16_vec(n, i).map(ShortVec), "short"),
            (LONG, 1) => e(p.u32(i).map(Long), "long"),
            (LONG, n) => e(p.u32_vec(n, i).map(LongVec), "long"),
            (LONG8, 1) => e(p.u64(i).map(Long8), "long8"),
            (LONG8, n) => e(p.u64_vec(n, i).map(Long8Vec), "long8"),
            (IFD8, 1) => e(p.u64(i).map(Ifd8), "ifd8"),
            (IFD8, n) => e(p.u64_vec(n, i).map(Ifd8Vec), "ifd8"),
            (ASCII, n) => e(p.ascii(n, i).map(Ascii), "ascii"),
            (RATIONAL, 1) => e(
                p.rational(i).map(|(n, d)| Rational { numer: n, denom: d }),
                "rational",
            ),
            (RATIONAL, c) => e(p.rational_vec(c, i).map(RationalVec), "rational"),
            (dt, _) => e(None, &format!("data-type {:?} is not supported", dt)),
        }
    }
}

#[cfg(test)]
mod tests_tiff_parser {
    use super::*;

    #[test]
    fn test_header_classic_intel() {
        let buf = [0x49, 0x49, 0x2A, 0x00, 0x09, 0x00, 0x00, 0x00];
        let result = Parser::header(&buf);
        match result {
            Ok((parser, next_ifd)) => {
                assert_eq!(parser, Classic(Intel));
                assert_eq!(next_ifd, 9);
            }
            _ => {
                assert!(false, "Failed to parse header");
            }
        }
    }
}
#[allow(dead_code)]
impl Bytes {
    fn ascii(&self, n: u64, i: &[u8]) -> Option<String> {
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

    fn rational(&self, i: &[u8]) -> Option<(u32, u32)> {
        if i.len() >= 8 {
            match (self.u32(&i[0..4]), self.u32(&i[4..8])) {
                (Some(numer), Some(denom)) => Some((numer, denom)),
                _ => None,
            }
        } else {
            None
        }
    }

    fn rational_vec(&self, n: u64, i: &[u8]) -> Option<Vec<(u32, u32)>> {
        let n = n as usize;
        if i.len() < n * 2 {
            return None;
        }
        Some(i[0..8 * n].chunks(8).fold(Vec::new(), |mut acc, x| {
            acc.push(self.rational(x).unwrap());
            acc
        }))
    }

    fn u8(&self, i: &[u8]) -> Option<u8> {
        if i.len() < 1 {
            None
        } else {
            Some(i[0])
        }
    }

    fn u8_vec(&self, n: u64, i: &[u8]) -> Option<Vec<u8>> {
        let n = n as usize;
        if i.len() < n {
            return None;
        } else {
            Some(i[0..n].to_vec())
        }
    }

    fn u16(&self, i: &[u8]) -> Option<u16> {
        if i.len() < 2 {
            return None;
        }
        match *self {
            Intel => Some(((i[1] as u16) << 8) + i[0] as u16),
            Moto => Some(((i[0] as u16) << 8) + i[1] as u16),
        }
    }

    fn u16_vec(&self, n: u64, i: &[u8]) -> Option<Vec<u16>> {
        let n = n as usize;
        if i.len() < n * 2 {
            return None;
        }
        Some(i[0..2 * n].chunks(2).fold(Vec::new(), |mut acc, x| {
            acc.push(self.u16(x).unwrap());
            acc
        }))
    }

    pub(crate) fn u32(&self, i: &[u8]) -> Option<u32> {
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

    fn u32_vec(&self, n: u64, i: &[u8]) -> Option<Vec<u32>> {
        let n = n as usize;
        if i.len() < n * 4 {
            return None;
        }
        Some(i[0..4 * n].chunks(4).fold(Vec::new(), |mut acc, x| {
            acc.push(self.u32(x).unwrap());
            acc
        }))
    }

    pub(crate) fn u64(&self, i: &[u8]) -> Option<u64> {
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

    fn u64_vec(&self, n: u64, i: &[u8]) -> Option<Vec<u64>> {
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
