use crate::base::{
    ReadCommand::{self, *},
    ReadConsumer,
};
use crate::error::EozinError;
use crate::tiff_tools::{
    bytes_parser::{BytesParser, LittleEndian},
    data::{DataType, Entry, Ifd},
    decode::{TiffResult, parse_data},
    error::TiffError::*,
};
/*
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
*/

// const MAX_ALLOCATION: u64 = 2 * 1024 * 1024 * 1024;

type B = LittleEndian;

#[derive(Debug)]
pub(crate) struct NdpiBigIfdDecoder {
    proc: DecodingProc,
    directories: Vec<Ifd>,
    entries: Vec<Entry>,
    remaining: Vec<RequestingEntry>,
}

#[derive(Debug)]
enum DecodingProc {
    RequestingHeader,
    RequestingIfdCount {
        offset: u64,
    },
    RequestingIfdBody {
        entry_offset: u64,
        entry_count: u64,
        // higher_offset: u32,
    },
    RequestingNextIfdOffset,
    RequestingEntry {
        target_dt: DataType,
        target_count: u64,
        target_tag: u16,
        entry_offset: u64,
        entry_count: u64,
    },
    Finish,
}

#[derive(Debug)]
enum EntryProc {
    InProcess(RequestingEntry),
    Finished(Entry),
}

#[derive(Debug)]
struct RequestingEntry {
    offset: u64,
    len: u64,
    tag: u16,
    dt: DataType,
    count: u64,
}

impl ReadConsumer for NdpiBigIfdDecoder {
    type Input = ();
    type Output = Vec<Ifd>;
    type ErrorKind = EozinError;
    fn dispatch(_input: Self::Input) -> (Self, ReadCommand) {
        let decoder = NdpiBigIfdDecoder {
            proc: DecodingProc::RequestingHeader,
            directories: Vec::new(),
            entries: Vec::new(),
            remaining: Vec::new(),
        };
        (
            decoder,
            ReadBytesStep {
                offset: 0,
                count: 12,
            },
        )
    }
    fn receive(self, _buf: &[u8]) -> Result<Vec<Ifd>, EozinError> {
        match self.proc {
            DecodingProc::Finish => Ok(self.directories),
            _ => Err(InvalidDecodingProcess("Invalid"))?,
        }
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, EozinError> {
        use self::DecodingProc::*;
        match self.proc {
            RequestingHeader => {
                /*
                let next_p = Self::next_ifd_from_file_header(buf).ok_or(ParseHeaderError(""))?;
                if next_p == 0 {
                    Err(InvalidDecodingProcess("Invalid"))
                } else {
                    self.proc = RequestingIfdCount { offset: next_p };
                    Ok(ReadBytesStep {
                        offset: next_p,
                        count: self.size.ifd_count,
                    })
                }
                */
                match (buf[0], buf[1], buf[2], buf[3]) {
                    (0x49, 0x49, 0x2A, 0x00) => {
                        // println!("Classic tiff")
                    }
                    _ => return Err(InvalidDecodingProcess("Invalid Header"))?,
                };
                let next_p = B::u64(&buf[4..12])
                    .and_then(|x| match x {
                        0 => None,
                        _ => Some(x),
                    })
                    .ok_or(ParseHeaderError(""))?;
                // println!("Next pointer {}", next_p);
                self.proc = RequestingIfdCount { offset: next_p };
                Ok(ReadBytesStep {
                    offset: next_p,
                    count: 2,
                })
                // Err(InvalidDecodingProcess("Invalid"))?
            }
            RequestingIfdCount { offset } => {
                /*

                let entry_count = T::ifd_entry_count(buf)?;
                // let buf_len = self.size.ifd_entry * entry_count;
                let buf_len = self
                    .size
                    .ifd_entry
                    .checked_mul(entry_count)
                    .ok_or(InvalidDecodingProcess("Buffer length overflow"))?;
                self.proc = RequestingIfdBody {
                    entry_offset: offset,
                    entry_count,
                };
                Ok(ReadBytesStep {
                    offset: offset + self.size.ifd_count,
                    count: buf_len,
                })
                */
                /*
                let entry_count = B::u16(buf)
                    .and_then(|x| match x {
                        0 => None,
                        _ => Some(x as u64),
                    })
                    .ok_or(DecodeCountError {
                        msg: "Failed to parse IFD count",
                    })?;
                let higher_offset = B::u32(&buf[2..6]).ok_or(DecodeCountError {
                    msg: "Failed to parse IFD count",
                })?;
                */
                let entry_count = B::u16(buf)
                    .and_then(|x| match x {
                        0 => None,
                        _ => Some(x as u64),
                    })
                    .ok_or(DecodeCountError {
                        msg: "Failed to parse IFD count",
                    })?;
                // MEMO: Entry Length (12) + Higher Offseet (4) = 16
                // MEMO: NEXT IFD Offset (8)
                let buf_len = 16_u64
                    .checked_mul(entry_count)
                    .and_then(|x| x.checked_add(8))
                    .ok_or(InvalidDecodingProcess("Buffer length overflow"))?;
                // println!("IFD count {}", entry_count);
                // println!("Higher offset {}", higher_offset);
                // Err(InvalidDecodingProcess("Invalid"))?
                self.proc = RequestingIfdBody {
                    entry_offset: offset,
                    entry_count,
                };
                Ok(ReadBytesStep {
                    offset: offset + 2,
                    count: buf_len,
                })
            }
            RequestingIfdBody {
                entry_count,
                entry_offset,
            } => {
                // println!("Entry Offset {}", entry_offset);
                self.entries.clear();
                self.remaining.clear();
                let len_entries: usize = (12_u64 * entry_count)
                    .try_into()
                    .map_err(|_| InvalidDecodingProcess("Buffer length overflow"))?;
                // let higher_offset = B::u32(&buf[l + 8..]);
                // println!("Higher offset {:?}", higher_offset);
                for (b, hb) in buf[..len_entries]
                    .chunks_exact(12)
                    .zip(buf[len_entries + 8..].chunks_exact(4))
                {
                    let ent = Self::entry(b, hb)?;
                    // println!("Ent {:?}", &ent);
                    match ent {
                        EntryProc::InProcess(p) => {
                            // println!("Pointer {:?}", p);
                            self.remaining.push(p);
                        }
                        EntryProc::Finished(e) => {
                            self.entries.push(e);
                        }
                    }
                }
                // println!("Entries {:?}", self.entries);
                if self.remaining.is_empty() {
                    self.proc = RequestingNextIfdOffset;
                    let next_ifd_ofs = entry_offset + 12 * entry_count + 6;
                    let entries = std::mem::take(&mut self.entries);
                    let ifd = Ifd::from_entries(entries);
                    self.directories.push(ifd);
                    Ok(ReadBytesStep {
                        offset: next_ifd_ofs,
                        count: 8,
                    })
                } else {
                    let e = self.remaining.pop().unwrap();
                    self.proc = RequestingEntry {
                        target_dt: e.dt,
                        target_count: e.count,
                        target_tag: e.tag,
                        entry_count,
                        entry_offset,
                    };
                    // println!("tag {}, Offset {} and len {}", e.tag, e.offset, e.len);
                    Ok(ReadBytesStep {
                        offset: e.offset,
                        count: e.len,
                    })
                }
            }
            RequestingEntry {
                target_tag,
                target_count,
                target_dt,
                entry_offset,
                entry_count,
            } => {
                /*
                let data = T::parse_data(buf, target_count as usize, target_dt)?;
                let ent = Entry {
                    tag: target_tag,
                    count: target_count,
                    data,
                };
                self.entries.push(ent);
                if self.remaining.is_empty() {
                    self.proc = RequestingNextIfdOffset;
                    let next_ifd_ofs =
                        entry_offset + self.size.ifd_entry * entry_count + self.size.ifd_count;
                    let entries = std::mem::take(&mut self.entries);
                    let ifd = Ifd::from_entries(entries);
                    self.directories.push(ifd);
                    Ok(ReadBytesStep {
                        offset: next_ifd_ofs,
                        count: self.size.ifd_pointer,
                    })
                } else {
                    let e = self.remaining.pop().unwrap();
                    self.proc = RequestingEntry {
                        target_dt: e.dt,
                        target_count: e.count,
                        target_tag: e.tag,
                        entry_count,
                        entry_offset,
                    };
                    Ok(ReadBytesStep {
                        offset: e.offset,
                        count: e.len,
                    })
                }
                */
                let data = parse_data::<B>(buf, target_count as usize, target_dt)?;
                /*
                println!(
                    "Requested {:?}, txt {:?}, v {:?}",
                    CommonTag::from(target_tag),
                    data.expect_ascii(),
                    data.to_u64()
                );
                */
                /*
                if target_tag == 65426 {
                    println!("Lower {:?}", &data.expect_u32vec().unwrap()[..10]);
                }
                if target_tag == 65432 {
                    println!("Higher {:?}", &data.expect_u32vec().unwrap()[..10]);
                }
                */
                let ent = Entry {
                    tag: target_tag,
                    count: target_count,
                    data,
                };
                self.entries.push(ent);
                if self.remaining.is_empty() {
                    self.proc = RequestingNextIfdOffset;
                    // let next_ifd_ofs = entry_offset + 12 * entry_count + 6;
                    let next_ifd_ofs = entry_offset + 12 * entry_count + 2;
                    let entries = std::mem::take(&mut self.entries);
                    let ifd = Ifd::from_entries(entries);
                    self.directories.push(ifd);
                    Ok(ReadBytesStep {
                        offset: next_ifd_ofs,
                        count: 8,
                    })
                } else {
                    let e = self.remaining.pop().unwrap();
                    self.proc = RequestingEntry {
                        target_dt: e.dt,
                        target_count: e.count,
                        target_tag: e.tag,
                        entry_count,
                        entry_offset,
                    };
                    /*
                    if e.tag == 65426 || e.tag == 65432 {
                        println!("tag {}, Offset {} and len {}", e.tag, e.offset, e.len);
                    }
                    */
                    Ok(ReadBytesStep {
                        offset: e.offset,
                        count: e.len,
                    })
                }
            }
            RequestingNextIfdOffset => {
                /*
                let next_ifd = T::next_ifd_pointer(buf);
                if let Some(offset) = next_ifd {
                    self.proc = RequestingIfdCount { offset };
                    Ok(ReadBytesStep {
                        offset,
                        count: self.size.ifd_count,
                    })
                } else {
                    self.proc = Finish;
                    Ok(NoCmd)
                }
                */
                match B::u64(&buf[0..8]).and_then(|x| match x {
                    0 => None,
                    _ => Some(x),
                }) {
                    Some(offset) => {
                        // println!("Having next ifd pointer {}", offset);
                        self.proc = RequestingIfdCount { offset };
                        Ok(ReadBytesStep { offset, count: 2 })
                    }
                    None => {
                        self.proc = Finish;
                        Ok(NoCmd)
                    }
                }
            }
            _ => Err(InvalidDecodingProcess("Invalid"))?,
        }
    }
}

impl NdpiBigIfdDecoder {
    fn entry(buf: &[u8], higher_offsets: &[u8]) -> TiffResult<EntryProc> {
        let tag = B::u16(&buf[..2]).ok_or(DecodeEntryError {
            msg: "Cannot parse entry tag",
        })?;
        let dt = B::u16(&buf[2..4])
            .and_then(DataType::from_u16)
            .ok_or(DecodeEntryError {
                msg: "Cannot parse entry data-type",
            })?;
        let count = B::u32(&buf[4..8])
            .map(|x| x as u64)
            .ok_or(DecodeEntryError {
                msg: "Cannot parse entry count",
            })?;
        let len = dt
            .size()
            .checked_mul(count)
            .ok_or(InvalidDecodingProcess(""))?;
        if len <= 4 {
            let content = if higher_offsets.iter().any(|&x| x != 0) && dt == DataType::LONG {
                let new_buf = [&buf[8..12], higher_offsets].concat();
                parse_data::<B>(&new_buf, count as usize, DataType::LONG8)
            } else {
                parse_data::<B>(&buf[8..12], count as usize, dt)
            }?;
            // let content = parse_data::<B>(&buf[8..12], count as usize, dt)?;
            Ok(EntryProc::Finished(Entry {
                tag,
                count,
                data: content,
            }))
        } else {
            let offset = u64::from_le_bytes([
                buf[8],
                buf[9],
                buf[10],
                buf[11],
                higher_offsets[0],
                higher_offsets[1],
                higher_offsets[2],
                higher_offsets[3],
            ]);
            /*
            let offset = B::u32(&buf[8..12])
                .map(|x| x as u64)
                .ok_or(DecodeEntryError {
                    msg: "Cannot parse entry data offset",
                })?;
            */
            // let offset: u64 = ((higher_offset as u64) << 32) | (offset as u64);
            Ok(EntryProc::InProcess(RequestingEntry {
                offset,
                count,
                len,
                dt,
                tag,
            }))
        }
    }
}

/*
fn run() -> Result<(), EozinError> {
    use crate::tiff_tools::tags::CommonTag::*;
    let path = "tests/openslideorg-images/Hamamatsu-1.ndpi";
    println!("file exists {}", Path::new(path).exists());
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);

    let buf = read_bytes(&mut reader, 0, 4).unwrap();
    println!("Buf {:?}", &buf);
    match (buf[0], buf[1], buf[2], buf[3]) {
        (0x49, 0x49, 0x2A, 0x00) => {
            println!("Classic tiff")
        }
        _ => return Err(InvalidDecodingProcess("Invalid Header"))?,
    }
    let decoder = excecute_read::<_, NdpiBigIfdDecoder>(&mut reader, ())?;
    // println!("Dec {:?}", decoder);
    for (i, ifd) in decoder.iter().enumerate() {
        println!("Lv {}", i);
        println!("Width {:?}", ifd.get(ImageWidth));
        println!("TileWidth {:?}", ifd.get(TileWidth));
        println!("StripOffset {:?}", ifd.get(StripOffsets));
        println!("StripCount {:?}", ifd.get(StripByteCounts));
        // println!("ImageDescription {:?}", ifd.get(ImageDescription));
        // println!("Software {:?}", ifd.get(Software));
    }
    Ok(())
}
*/

/*
fn excecute_read<R, C>(mut r: &mut R, input: C::Input) -> Result<C::Output, EozinError>
where
    R: Read + Seek,
    C: ReadConsumer<ErrorKind = EozinError>,
{
    let (mut c, mut cmd) = C::dispatch(input);
    for _ in 0..10000 {
        match cmd {
            NoCmd => return c.receive(&[]),
            ReadBytes { offset, count } => {
                let buf = read_bytes(&mut r, offset, count)?;
                return c.receive(&buf);
            }
            ReadBytesStep { offset, count } => {
                let buf = read_bytes(&mut r, offset, count)?;
                cmd = c.step(&buf)?;
            }
        }
    }
    Err(UnableToAllocateLargeSize)
}
*/

/*
fn read_bytes<R: Read + Seek>(
    data: &mut R,
    offset: u64,
    count: u64,
) -> Result<Vec<u8>, EozinError> {
    if count > MAX_ALLOCATION {
        return Err(UnableToAllocateLargeSize);
    }
    let mut buffer = vec![0_u8; count as usize];
    data.seek(SeekFrom::Start(offset))?;
    let l = data.read(&mut buffer)?;
    if l == count as usize {
        Ok(buffer)
    } else {
        Err(UnexpectedStep)
    }
}
*/

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Copy, Clone, Eq)]
enum NdpiTag {
    RestartMarkerLowerBytes,
    RestartMarkerHigherBytes,
    SourceLens,             // 65421 SourceLens
    XOffsetFromSlideCenter, // 65422 	XOffsetFromSlideCentre
    YOffsetFromSlideCenter, // 65423 	YOffsetFromSlideCentre
}

impl From<NdpiTag> for u16 {
    fn from(tag: NdpiTag) -> u16 {
        use NdpiTag::*;
        match tag {
            RestartMarkerLowerBytes => 65426,
            RestartMarkerHigherBytes => 65432,
            SourceLens => 65421,
            XOffsetFromSlideCenter => 65422,
            YOffsetFromSlideCenter => 65423,
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    /*
    #[test]
    fn ndpi_big() {
        // run().unwrap();
        assert_eq!(4, 4);
    }
    */
}
