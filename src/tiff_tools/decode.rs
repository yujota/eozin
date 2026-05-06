use super::bytes_parser::{BigEndian, BytesParser, LittleEndian};
use super::data::{
    Data::{self, *},
    DataType::{self, *},
    Entry, Ifd,
};
use super::error::TiffError::{self, *};
use std::marker::PhantomData;

use crate::base::{
    ReadCommand::{self, *},
    ReadConsumer,
};

pub(crate) type TiffResult<T> = Result<T, TiffError>;

#[allow(private_interfaces)]
#[derive(Debug)]
pub(crate) enum Decoder {
    Init,
    ClassicLittle(TiffDecoder<ClassicTiffParser<LittleEndian>, LittleEndian>),
    ClassicBig(TiffDecoder<ClassicTiffParser<BigEndian>, BigEndian>),
    BigTiffLittle(TiffDecoder<BigTiffParser<LittleEndian>, LittleEndian>),
    BigTiffBig(TiffDecoder<BigTiffParser<BigEndian>, BigEndian>),
}

impl ReadConsumer for Decoder {
    type Input = ();
    type Output = Vec<Ifd>;
    type ErrorKind = TiffError;

    fn dispatch(_input: Self::Input) -> (Self, ReadCommand) {
        (
            Decoder::Init,
            ReadBytesStep {
                offset: 0,
                count: 4,
            },
        )
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        use self::Decoder::*;
        match self {
            Init => Err(InvalidDecodingProcess("Invalid Header")),
            ClassicBig(p) => p.receive(buf),
            ClassicLittle(p) => p.receive(buf),
            BigTiffBig(p) => p.receive(buf),
            BigTiffLittle(p) => p.receive(buf),
        }
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        use self::Decoder::*;
        match &mut *self {
            Init => {
                if buf.len() < 4 {
                    return Err(InvalidDecodingProcess("Buf size not enough"));
                }
                match (buf[0], buf[1], buf[2], buf[3]) {
                    (0x4D, 0x4D, 0x00, 0x2A) => {
                        let mut p = TiffDecoder::classic_big();
                        let r = p.dispatch();
                        *self = ClassicBig(p);
                        Ok(r)
                    }
                    (0x4D, 0x4D, 0x00, 0x2B) => {
                        let mut p = TiffDecoder::bigtiff_big();
                        let r = p.dispatch();
                        *self = BigTiffBig(p);
                        Ok(r)
                    }
                    (0x49, 0x49, 0x2A, 0x00) => {
                        let mut p = TiffDecoder::classic_little();
                        let r = p.dispatch();
                        *self = ClassicLittle(p);
                        Ok(r)
                    }
                    (0x49, 0x49, 0x2B, 0x00) => {
                        let mut p = TiffDecoder::bigtiff_little();
                        let r = p.dispatch();
                        *self = BigTiffLittle(p);
                        Ok(r)
                    }
                    _ => Err(InvalidDecodingProcess("Invalid Header")),
                }
            }
            ClassicBig(p) => p.step(buf),
            ClassicLittle(p) => p.step(buf),
            BigTiffBig(p) => p.step(buf),
            BigTiffLittle(p) => p.step(buf),
        }
    }
}

#[derive(Debug)]
struct TiffDecoder<T, B>
where
    T: TiffParser<B>,
    B: BytesParser,
{
    _t: PhantomData<T>,
    _b: PhantomData<B>,
    size: Size,
    proc: DecodingProc,
    directories: Vec<Ifd>,
    entries: Vec<Entry>,
    remaining: Vec<RequestingEntry>,
}

impl TiffDecoder<ClassicTiffParser<LittleEndian>, LittleEndian> {
    fn classic_little() -> TiffDecoder<ClassicTiffParser<LittleEndian>, LittleEndian> {
        TiffDecoder {
            _t: PhantomData,
            _b: PhantomData,
            size: Size::classic(),
            proc: DecodingProc::Init,
            directories: Vec::new(),
            entries: Vec::new(),
            remaining: Vec::new(),
        }
    }
}

impl TiffDecoder<ClassicTiffParser<BigEndian>, BigEndian> {
    fn classic_big() -> TiffDecoder<ClassicTiffParser<BigEndian>, BigEndian> {
        TiffDecoder {
            _t: PhantomData,
            _b: PhantomData,
            size: Size::classic(),
            proc: DecodingProc::Init,
            directories: Vec::new(),
            entries: Vec::new(),
            remaining: Vec::new(),
        }
    }
}

impl TiffDecoder<BigTiffParser<LittleEndian>, LittleEndian> {
    fn bigtiff_little() -> TiffDecoder<BigTiffParser<LittleEndian>, LittleEndian> {
        TiffDecoder {
            _t: PhantomData,
            _b: PhantomData,
            size: Size::big(),
            proc: DecodingProc::Init,
            directories: Vec::new(),
            entries: Vec::new(),
            remaining: Vec::new(),
        }
    }
}

impl TiffDecoder<BigTiffParser<BigEndian>, BigEndian> {
    fn bigtiff_big() -> TiffDecoder<BigTiffParser<BigEndian>, BigEndian> {
        TiffDecoder {
            _t: PhantomData,
            _b: PhantomData,
            size: Size::big(),
            proc: DecodingProc::Init,
            directories: Vec::new(),
            entries: Vec::new(),
            remaining: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Size {
    pub file_header: u64,
    pub ifd_count: u64,
    pub ifd_entry: u64,
    pub ifd_pointer: u64,
}

impl Size {
    pub fn classic() -> Self {
        Self {
            file_header: 8,
            ifd_count: 2,
            ifd_entry: 12,
            ifd_pointer: 4,
        }
    }

    pub fn big() -> Self {
        Self {
            file_header: 16,
            ifd_count: 8,
            ifd_entry: 20,
            ifd_pointer: 8,
        }
    }
}

#[derive(Debug)]
pub(crate) enum DecodingProc {
    Init,
    RequestingHeader,
    RequestingIfdCount {
        offset: u64,
    },
    RequestingIfdBody {
        entry_offset: u64,
        entry_count: u64,
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
pub(crate) enum EntryProc {
    InProcess(RequestingEntry),
    Finished(Entry),
}

#[derive(Debug)]
pub(crate) struct RequestingEntry {
    offset: u64,
    len: u64,
    tag: u16,
    dt: DataType,
    count: u64,
}

#[derive(Debug)]
struct BigTiffParser<B: BytesParser> {
    _bp: PhantomData<B>,
}

#[derive(Debug)]
struct ClassicTiffParser<B: BytesParser> {
    _bp: PhantomData<B>,
}

impl<T, B> TiffDecoder<T, B>
where
    T: TiffParser<B>,
    B: BytesParser,
{
    fn dispatch(&mut self) -> ReadCommand {
        self.proc = DecodingProc::RequestingHeader;
        ReadBytesStep {
            offset: 0,
            count: self.size.file_header,
        }
    }
    fn receive(self, _buf: &[u8]) -> Result<Vec<Ifd>, TiffError> {
        match self.proc {
            DecodingProc::Finish => Ok(self.directories),
            _ => Err(InvalidDecodingProcess("Invalid")),
        }
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, TiffError> {
        use self::DecodingProc::*;
        match self.proc {
            Init => Err(InvalidDecodingProcess("Invalid")),
            RequestingHeader => {
                let next_p = T::next_ifd_from_file_header(buf).ok_or(ParseHeaderError(""))?;
                if next_p == 0 {
                    Err(InvalidDecodingProcess("Invalid"))
                } else {
                    self.proc = RequestingIfdCount { offset: next_p };
                    Ok(ReadBytesStep {
                        offset: next_p,
                        count: self.size.ifd_count,
                    })
                }
            }
            RequestingIfdCount { offset } => {
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
            }
            RequestingIfdBody {
                entry_count,
                entry_offset,
            } => {
                self.entries.clear();
                self.remaining.clear();
                for b in buf.chunks(self.size.ifd_entry as usize) {
                    let ent = match T::entry(b) {
                        Ok(e) => e,
                        Err(e) => {
                            println!("Err {:?}", e);
                            return Err(e);
                        }
                    };
                    match ent {
                        EntryProc::InProcess(p) => {
                            self.remaining.push(p);
                        }
                        EntryProc::Finished(e) => {
                            self.entries.push(e);
                        }
                    }
                }
                if self.remaining.is_empty() {
                    println!("Empty!!!");
                    self.proc = RequestingNextIfdOffset;
                    // let next_ifd_ofs = entry_offset + self.size.ifd_entry * entry_count;
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
            }
            RequestingEntry {
                target_tag,
                target_count,
                target_dt,
                entry_offset,
                entry_count,
            } => {
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
            }
            RequestingNextIfdOffset => {
                let next_ifd = T::next_ifd_pointer(buf);
                // let next_ifd = Some(T::next_ifd_pointer(buf).unwrap_or(0));
                /*
                println!(
                    "Next IFD {:?}, {}, {:?}",
                    next_ifd,
                    self.directories.len(),
                    &buf
                );
                */
                if let Some(offset) = next_ifd {
                    self.proc = RequestingIfdCount { offset };
                    Ok(ReadBytesStep {
                        offset,
                        count: self.size.ifd_count,
                    })
                } else {
                    /*
                    for ifd in self.directories.iter() {
                        use super::tags::{CommonTag::*, NdpiTag::*};
                        println!(
                            "IFD Width {:?}, {:?}",
                            ifd.get(ImageWidth),
                            ifd.get(MetadataBlock)
                        );
                    }
                    */
                    self.proc = Finish;
                    Ok(NoCmd)
                }
            }
            _ => Err(InvalidDecodingProcess("Invalid")),
        }
    }
}

pub trait TiffParser<B: BytesParser> {
    fn next_ifd_from_file_header(buf: &[u8]) -> Option<u64>
    where
        Self: Sized;
    fn next_ifd_pointer(buf: &[u8]) -> Option<u64>
    where
        Self: Sized;
    fn ifd_entry_count(buf: &[u8]) -> TiffResult<u64>
    where
        Self: Sized;
    fn entry(buf: &[u8]) -> TiffResult<EntryProc>
    where
        Self: Sized;
    fn parse_data(buf: &[u8], count: usize, data_type: DataType) -> TiffResult<Data>
    where
        Self: Sized,
    {
        parse_data::<B>(buf, count, data_type)
    }
}

pub(crate) fn parse_data<B: BytesParser>(
    buf: &[u8],
    count: usize,
    data_type: DataType,
) -> TiffResult<Data> {
    fn r_er(m: Option<Data>, dt: DataType, count: usize) -> TiffResult<Data> {
        m.ok_or(DecodeDataError { dt, count, msg: "" })
    }
    match (data_type, count) {
        (BYTE, 1) => r_er(B::u8(buf).map(Byte), BYTE, 1),
        (BYTE, c) => r_er(B::u8_vec(buf).map(ByteVec), BYTE, c),
        (SBYTE, 1) => r_er(B::i8(buf).map(SByte), SBYTE, 1),
        (SBYTE, c) => r_er(B::i8_vec(buf).map(SByteVec), SBYTE, c),
        (UNDEFINED, 1) => r_er(B::u8(buf).map(Undefined), UNDEFINED, 1),
        (UNDEFINED, c) => r_er(B::u8_vec(buf).map(UndefinedVec), UNDEFINED, c),
        (SHORT, 1) => r_er(B::u16(buf).map(Short), SHORT, 1),
        (SHORT, c) => r_er(B::u16_vec(buf).map(ShortVec), SHORT, c),
        (LONG, 1) => r_er(B::u32(buf).map(Long), LONG, 1),
        (LONG, c) => r_er(B::u32_vec(buf).map(LongVec), LONG, c),
        (SLONG, 1) => r_er(B::i32(buf).map(SLong), SLONG, 1),
        (SLONG, c) => r_er(B::i32_vec(buf).map(SLongVec), SLONG, c),
        (LONG8, 1) => r_er(B::u64(buf).map(Long8), LONG8, 1),
        (LONG8, c) => r_er(B::u64_vec(buf).map(Long8Vec), LONG8, c),
        (ASCII, c) => r_er(B::ascii(buf).map(Ascii), ASCII, c),
        (RATIONAL, 1) => r_er(
            B::rational(buf).map(|(n, d)| Rational { numer: n, denom: d }),
            RATIONAL,
            1,
        ),
        (RATIONAL, c) => r_er(B::rational_vec(buf).map(RationalVec), RATIONAL, c),
        (FLOAT, 1) => r_er(B::f32(buf).map(Float), FLOAT, 1),
        (FLOAT, c) => r_er(B::f32_vec(buf).map(FloatVec), FLOAT, c),
        (DOUBLE, 1) => r_er(B::f64(buf).map(Double), DOUBLE, 1),
        (DOUBLE, c) => r_er(B::f64_vec(buf).map(DoubleVec), DOUBLE, c),
        (dt, _) => Err(UnsupportedDataType { dt, msg: "" }),
    }
}

impl<B: BytesParser> TiffParser<B> for ClassicTiffParser<B> {
    fn next_ifd_from_file_header(buf: &[u8]) -> Option<u64> {
        B::u32(&buf[4..8]).and_then(|x| match x {
            0 => None,
            _ => Some(x as u64),
        })
    }
    fn next_ifd_pointer(buf: &[u8]) -> Option<u64> {
        B::u32(buf).and_then(|x| match x {
            0 => None,
            _ => Some(x as u64),
        })
    }
    fn ifd_entry_count(buf: &[u8]) -> TiffResult<u64> {
        B::u16(buf)
            .and_then(|x| match x {
                0 => None,
                _ => Some(x as u64),
            })
            .ok_or(DecodeCountError {
                msg: "Failed to parse IFD count",
            })
    }
    fn entry(buf: &[u8]) -> TiffResult<EntryProc> {
        let tag = B::u16(&buf[..2]).ok_or(DecodeEntryError {
            msg: "Cannot parse entry tag",
        })?;
        let dt = B::u16(&buf[2..4])
            .and_then(DataType::from_u16)
            .ok_or(DecodeEntryError {
                msg: "Cannot parse entry data-type",
            })?;
        let count = B::u32(&buf[4..8]).ok_or(DecodeEntryError {
            msg: "Cannot parse entry count",
        })?;
        let len = dt.size() * (count as u64);
        if len <= 4 {
            let content = Self::parse_data(&buf[8..12], count as usize, dt)?;
            Ok(EntryProc::Finished(Entry {
                tag,
                count: count as u64,
                data: content,
            }))
        } else {
            let offset = B::u32(&buf[8..12]).ok_or(DecodeEntryError {
                msg: "Cannot parse entry data offset",
            })?;
            Ok(EntryProc::InProcess(RequestingEntry {
                offset: offset as u64,
                count: count as u64,
                len,
                dt,
                tag,
            }))
        }
    }
}

impl<B: BytesParser> TiffParser<B> for BigTiffParser<B> {
    fn next_ifd_from_file_header(buf: &[u8]) -> Option<u64> {
        let buf = &buf[4..16];
        let (always_8, always_0) = (B::u16(&buf[..2]), B::u16(&buf[2..4]));
        match (always_8, always_0) {
            (Some(8), Some(0)) => B::u64(&buf[4..12]).and_then(|x| match x {
                0 => None,
                _ => Some(x),
            }),
            _ => None,
        }
    }
    fn next_ifd_pointer(buf: &[u8]) -> Option<u64> {
        B::u64(buf).and_then(|x| match x {
            0 => None,
            _ => Some(x),
        })
    }
    fn ifd_entry_count(buf: &[u8]) -> TiffResult<u64> {
        B::u64(buf).ok_or(DecodeCountError {
            msg: "Failed to parse IFD count",
        })
    }
    fn entry(buf: &[u8]) -> TiffResult<EntryProc> {
        let tag = B::u16(&buf[..2]).ok_or(DecodeEntryError {
            msg: "Cannot parse entry tag",
        })?;
        let dt = B::u16(&buf[2..4])
            .and_then(DataType::from_u16)
            .ok_or(DecodeEntryError {
                msg: "Cannot parse entry data-type",
            })?;
        let count = B::u64(&buf[4..12]).ok_or(DecodeEntryError {
            msg: "Cannot parse entry count",
        })?;
        // let len = dt.size() * count;
        let len = dt
            .size()
            .checked_mul(count)
            .ok_or(InvalidDecodingProcess("Buffer length overflow"))?;
        if len <= 8 {
            let content = Self::parse_data(&buf[12..20], count as usize, dt)?;
            Ok(EntryProc::Finished(Entry {
                tag,
                count,
                data: content,
            }))
        } else {
            let offset = B::u64(&buf[12..20]).ok_or(DecodeEntryError {
                msg: "Cannot parse entry data offset",
            })?;
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
