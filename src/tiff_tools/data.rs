use self::Data::*;
use self::DataType::*;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Ifd {
    pub entries: HashMap<u16, Data>,
}

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub tag: u16,
    pub count: u64,
    pub data: Data,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Clone)]
pub enum Data {
    Byte(u8),
    Ascii(String),
    Short(u16),
    Long(u32),
    Rational { numer: u32, denom: u32 },
    SByte(i8),
    Undefined(u8),
    SShort(i16),
    SLong(i32),
    SRational { numer: i32, denom: i32 },
    Float(f32),
    Double(f64),

    Long8(u64),
    SLong8(i64),
    Ifd8(u64),

    ByteVec(Vec<u8>),
    ShortVec(Vec<u16>),
    LongVec(Vec<u32>),
    RationalVec(Vec<(u32, u32)>), // 分子, 分母
    SByteVec(Vec<i8>),
    UndefinedVec(Vec<u8>),
    SShortVec(Vec<i16>),
    SLongVec(Vec<i32>),
    SRationalVec(Vec<(i32, i32)>),
    FloatVec(Vec<f32>),
    DoubleVec(Vec<f64>),

    Long8Vec(Vec<u64>),
    SLong8Vec(Vec<i64>),
    Ifd8Vec(Vec<u64>),
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DataType {
    BYTE, // uint8
    ASCII,
    SHORT, // uint16
    LONG,
    RATIONAL,
    SBYTE,
    UNDEFINED,
    SSHORT,
    SLONG,
    SRATIONAL,
    FLOAT,
    DOUBLE,
    LONG8,
    SLONG8,
    IFD8,
}

#[allow(dead_code)]
impl Data {
    pub fn get_datatype(&self) -> DataType {
        match self {
            Byte(_) => BYTE,
            Ascii(_) => ASCII,
            Short(_) => SHORT,
            Long(_) => LONG,
            Rational { .. } => RATIONAL,
            SByte(_) => SBYTE,
            Undefined(_) => UNDEFINED,
            SShort(_) => SSHORT,
            SLong(_) => SLONG,
            SRational { .. } => SRATIONAL,
            Float(_) => FLOAT,
            Double(_) => DOUBLE,
            Long8(_) => LONG8,
            SLong8(_) => SLONG8,
            Ifd8(_) => IFD8,

            ByteVec(_) => BYTE,
            ShortVec(_) => SHORT,
            LongVec(_) => LONG,
            RationalVec(_) => RATIONAL,
            SByteVec(_) => SBYTE,
            UndefinedVec(_) => UNDEFINED,
            SShortVec(_) => SSHORT,
            SLongVec(_) => SLONG,
            SRationalVec(_) => SRATIONAL,
            FloatVec(_) => FLOAT,
            DoubleVec(_) => DOUBLE,
            Long8Vec(_) => LONG8,
            SLong8Vec(_) => SLONG8,
            Ifd8Vec(_) => IFD8,
        }
    }

    pub fn to_u64(&self) -> Option<u64> {
        match self {
            Byte(x) => Some(*x as u64),
            Undefined(x) => Some(*x as u64),
            Short(x) => Some(*x as u64),
            Long(x) => Some(*x as u64),
            Long8(x) => Some(*x),
            _ => None,
        }
    }

    pub fn expect_u32vec(&self) -> Option<Vec<u32>> {
        match self {
            LongVec(xs) => Some(xs.clone()),
            _ => None,
        }
    }

    pub fn to_u64vec(&self) -> Option<Vec<u64>> {
        match self {
            Byte(x) => Some(vec![*x as u64]),
            Undefined(x) => Some(vec![*x as u64]),
            Short(x) => Some(vec![*x as u64]),
            Long(x) => Some(vec![*x as u64]),
            Long8(x) => Some(vec![*x]),
            ByteVec(xs) => Some(xs.iter().map(|x| *x as u64).collect()),
            UndefinedVec(xs) => Some(xs.iter().map(|x| *x as u64).collect()),
            ShortVec(xs) => Some(xs.iter().map(|x| *x as u64).collect()),
            LongVec(xs) => Some(xs.iter().map(|x| *x as u64).collect()),
            Long8Vec(xs) => Some(xs.clone()),
            _ => None,
        }
    }

    pub fn expect_u8vec(&self) -> Option<Vec<u8>> {
        match self {
            ByteVec(xs) => Some(xs.clone()),
            UndefinedVec(xs) => Some(xs.clone()),
            _ => None,
        }
    }

    pub fn expect_ascii(&self) -> Option<String> {
        match self {
            Ascii(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn count(&self) -> Option<u64> {
        #[allow(unreachable_patterns)]
        match self {
            Byte(_)
            | Short(_)
            | Long(_)
            | Rational { .. }
            | SByte(_)
            | Undefined(_)
            | SShort(_)
            | SLong(_)
            | SRational { .. }
            | Float(_)
            | Double(_)
            | Long8(_)
            | SLong8(_)
            | Ifd8(_) => Some(1),
            ByteVec(v) => Some(v.len() as u64),
            ShortVec(v) => Some(v.len() as u64),
            LongVec(v) => Some(v.len() as u64),
            RationalVec(v) => Some(v.len() as u64),
            SByteVec(v) => Some(v.len() as u64),
            UndefinedVec(v) => Some(v.len() as u64),
            SShortVec(v) => Some(v.len() as u64),
            SLongVec(v) => Some(v.len() as u64),
            SRationalVec(v) => Some(v.len() as u64),
            FloatVec(v) => Some(v.len() as u64),
            DoubleVec(v) => Some(v.len() as u64),
            Long8Vec(v) => Some(v.len() as u64),
            SLong8Vec(v) => Some(v.len() as u64),
            Ifd8Vec(v) => Some(v.len() as u64),
            Ascii(s) => Some(s.len() as u64),
            _ => None,
        }
    }

    pub fn encode_classic_intel(&self) -> Option<Vec<u8>> {
        #[allow(unreachable_patterns)]
        match self {
            Byte(x) => Some((*x).to_le_bytes().to_vec()),
            Short(x) => Some((*x).to_le_bytes().to_vec()),
            Long(x) => Some((*x).to_le_bytes().to_vec()),
            Rational { numer, denom } => {
                Some([numer.to_le_bytes(), denom.to_le_bytes()].concat().to_vec())
            }
            SByte(x) => Some((*x).to_le_bytes().to_vec()),
            Undefined(x) => Some((*x).to_le_bytes().to_vec()),
            SShort(x) => Some((*x).to_le_bytes().to_vec()),
            SLong(x) => Some((*x).to_le_bytes().to_vec()),
            SRational { numer, denom } => {
                Some([numer.to_le_bytes(), denom.to_le_bytes()].concat().to_vec())
            }
            Float(x) => Some((*x).to_le_bytes().to_vec()),
            Double(x) => Some((*x).to_le_bytes().to_vec()),
            ByteVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.to_le_bytes());
                acc
            })),
            ShortVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.to_le_bytes());
                acc
            })),
            LongVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.to_le_bytes());
                acc
            })),
            RationalVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.0.to_le_bytes());
                acc.extend_from_slice(&x.1.to_le_bytes());
                acc
            })),
            SByteVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.to_le_bytes());
                acc
            })),
            UndefinedVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.to_le_bytes());
                acc
            })),
            SShortVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.to_le_bytes());
                acc
            })),
            SLongVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.to_le_bytes());
                acc
            })),
            SRationalVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.0.to_le_bytes());
                acc.extend_from_slice(&x.1.to_le_bytes());
                acc
            })),
            FloatVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.to_le_bytes());
                acc
            })),
            DoubleVec(xs) => Some(xs.iter().fold(Vec::new(), |mut acc, x| {
                acc.extend_from_slice(&x.to_le_bytes());
                acc
            })),
            Ascii(s) => Some(s.as_bytes().to_vec()),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl DataType {
    pub fn from_u16(data_type: u16) -> Option<DataType> {
        match data_type {
            1 => Some(BYTE),
            2 => Some(ASCII),
            3 => Some(SHORT),
            4 => Some(LONG),
            5 => Some(RATIONAL),
            6 => Some(SBYTE),
            7 => Some(UNDEFINED),
            8 => Some(SSHORT),
            9 => Some(SLONG),
            10 => Some(SRATIONAL),
            11 => Some(FLOAT),
            12 => Some(DOUBLE),
            16 => Some(LONG8),
            17 => Some(SLONG8),
            18 => Some(IFD8),
            _ => {
                println!("Unknown data type {}", data_type);
                None
            }
        }
    }

    pub fn as_u16(&self) -> u16 {
        match self {
            BYTE => 1, // uint8
            ASCII => 2,
            SHORT => 3, // uint16
            LONG => 4,
            RATIONAL => 5,
            SBYTE => 6,
            UNDEFINED => 7,
            SSHORT => 8,
            SLONG => 9,
            SRATIONAL => 10,
            FLOAT => 11,
            DOUBLE => 12,
            LONG8 => 16,
            SLONG8 => 17,
            IFD8 => 18,
        }
    }

    pub fn size(&self) -> u64 {
        match self {
            BYTE | ASCII | SBYTE | UNDEFINED => 1,
            SHORT | SSHORT => 2,
            LONG | SLONG | FLOAT => 4,
            RATIONAL | SRATIONAL | DOUBLE | LONG8 | SLONG8 | IFD8 => 8,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<u16> for DataType {
    fn into(self) -> u16 {
        match self {
            BYTE => 1, // uint8
            ASCII => 2,
            SHORT => 3, // uint16
            LONG => 4,
            RATIONAL => 5,
            SBYTE => 6,
            UNDEFINED => 7,
            SSHORT => 8,
            SLONG => 9,
            SRATIONAL => 10,
            FLOAT => 11,
            DOUBLE => 12,
            LONG8 => 16,
            SLONG8 => 17,
            IFD8 => 18,
        }
    }
}

#[allow(clippy::from_over_into)]
#[allow(unreachable_patterns)]
impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            BYTE => "byte", // uint8
            ASCII => "ascii",
            SHORT => "short", // uint16
            LONG => "long",
            RATIONAL => "rational",
            SBYTE => "signed byte",
            UNDEFINED => "undefined",
            SSHORT => "signed short",
            SLONG => "signed long",
            SRATIONAL => "signed rational",
            FLOAT => "float",
            DOUBLE => "double",
            LONG8 => "long-8(u64)",
            SLONG8 => "singned-long-8(i64)",
            IFD8 => "ifd offset 8bit",
            _ => "Unknown data type",
        };
        write!(f, "{}", s).unwrap();
        Ok(())
    }
}

#[allow(dead_code)]
impl Ifd {
    pub fn from_entries(entries: Vec<Entry>) -> Ifd {
        let entries = entries
            .into_iter()
            .map(move |Entry { tag, data, .. }| (tag.into(), data))
            .collect();
        Ifd { entries }
    }

    pub fn remove<T: Into<u16>>(&mut self, tag: T) -> Option<Data> {
        self.entries.remove(&tag.into())
    }

    pub fn insert<T: Into<u16>>(&mut self, tag: T, data: Data) -> Option<Data> {
        self.entries.insert(tag.into(), data)
    }

    pub fn get<T: Into<u16>>(&self, tag: T) -> Option<&Data> {
        self.entries.get(&tag.into())
    }

    pub fn keys(&self) -> Vec<u16> {
        self.entries.keys().copied().collect()
    }
}
