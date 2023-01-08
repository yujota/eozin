use crate::tiff::error::*;
use crate::tiff::parser;
use crate::tiff::types::{BytesParser, BytesParser::*, Data, TiffParser, TiffParser::*};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

type Offset = u64;
type Tag = u16;
type IFD = HashMap<Tag, Data>;
type Tiff = Vec<IFD>;

pub(in crate::tiff) fn decode_file(mut file: File) -> Result<Tiff, TiffError> {
    // TODO: TiffError -> SomeError including some IO Error.
    let buf = read_bytes(&mut file, 0, 16).ok_or(format_error("ahh..io error"))?;
    let (p, ifd_offset) = decode_header(&buf)?;
    println!("parser:: {:?}", &p);
    let mut next_ifd = Some(ifd_offset);
    let mut directories = Vec::new();
    while let Some(ofs) = next_ifd {
        let buf = read_bytes(&mut file, ofs, ofs + 8).ok_or(format_error("io error 2"))?;
        let (cou, (b_start, b_end), (next_ofs, next_ofs_end)) = p.ifd_header(ofs, &buf)?;
        println!("======= {:?}  =======", (cou, ofs, b_start, next_ofs));
        let buf = read_bytes(&mut file, b_start, b_end).ok_or(format_error("io error 3"))?;
        let (mut entries, data_offsets, _errors) = p.ifd_body(&buf);
        let buf =
            read_bytes(&mut file, next_ofs, next_ofs_end).ok_or(format_error("io error 4"))?;
        println!("Entries! {:?}", &entries);
        for (tag, d) in data_offsets.into_iter() {
            println!("== tag {:?}, ofs {:?}", tag, d);
            let buf_size = d.data_type.size() * d.count;
            let buf = read_bytes(&mut file, d.offset, d.offset + buf_size)
                .ok_or(format_error("io error 5"))?;
            let data = p.entry_data(d.count, d.data_type, &buf)?;
            entries.insert(tag, data);
        }
        next_ifd = p.next_ifd(&buf)?;
        directories.push(entries)
    }
    Ok(directories)
}

fn decode_header(buf: &[u8]) -> Result<(TiffParser, Offset), TiffError> {
    let parser = match BytesParser::Moto.u16(&buf[0..2]).unwrap() {
        18761 => {
            // Little endian(Intel) 18761 == 0x49, 0x49
            BytesParser::Intel
        }
        19789 => {
            // Big endian(Motorola) 19789 == 0x4d, 0x4d
            BytesParser::Moto
        }
        _ => return Err(format_error("Unknown endian")),
    };
    match parser.u16(&buf[2..4]).unwrap() {
        42 => {
            let next_ifd = parser.u32(&buf[4..8]).unwrap() as u64;
            Ok((Classic(parser), next_ifd))
        }
        43 => {
            let buf = &buf[4..16];
            let (always_8, always_0) = (
                parser.u16(&buf[..2]).unwrap(),
                parser.u16(&buf[2..4]).unwrap(),
            );
            if always_8 == 8 && always_0 == 0 {
                let next_ifd = parser.u64(&buf[4..12]).unwrap();
                Ok((Big(parser), next_ifd))
            } else {
                Err(format_error("Unknown tiff version"))
            }
        }
        _ => Err(format_error("Unknown tiff version")),
    }
}

fn read_bytes(file: &mut File, start: u64, end: u64) -> Option<Vec<u8>> {
    let len = (end - start) as usize;
    let mut buffer = vec![0; len];
    file.seek(SeekFrom::Start(start)).ok()?;
    file.read(&mut buffer)
        .ok()
        .and_then(|x| if x == len { Some(buffer) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_classic_intel() {
        let buf = [0x49, 0x49, 0x2A, 0x00, 0x09, 0x00, 0x00, 0x00];
        let actual = decode_header(&buf);
        match actual {
            Ok((parser, next_ifd)) => {
                assert_eq!(parser, Classic(Intel));
                assert_eq!(next_ifd, 9);
            }
            _ => {
                assert!(false, "Failed to parse header");
            }
        }
    }

    #[test]
    fn test_decode_file() {
        let mut file = File::open("../data/CMU-1-Small-Region.svs").unwrap();
        let result = decode_file(file);
        match result {
            Ok(tiff) => {
                println!("{:?}", tiff);
            }
            Err(err) => {
                println!("{:?}", err);
                assert!(false, "Failed to decode file");
            }
        }
    }
}
