use std::collections::HashMap;

pub(crate) type Tiff = Vec<IFD>;
pub(crate) type Tag = u16;
pub(crate) type IFD = HashMap<Tag, Data>;

pub(crate) enum Entry {
    DataEntry(Data),
    OffsetEntry(DataOffset),
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct DataOffset {
    pub data_type: DataType,
    pub count: u64,
    pub offset: u64,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub(crate) enum Data {
    Byte(u8),
    Ascii(String),
    Short(u16),
    Long(u32),
    Rational { numer: u32, denom: u32 }, // 分子, 分母
    SByte(i8),
    Undefined(u8),
    SShort(i16),
    SLong(i32),
    SRational { numer: i32, denom: i32 }, // 分子, 分母
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
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum DataType {
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
impl DataType {
    pub fn from_u16(data_type: u16) -> Option<DataType> {
        use DataType::*;
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
            _ => None,
        }
    }

    pub fn as_u16(&self) -> u16 {
        use DataType::*;
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
        use DataType::*;
        match self {
            BYTE | ASCII | SBYTE | UNDEFINED => 1,
            SHORT | SSHORT => 2,
            LONG | SLONG | FLOAT => 4,
            RATIONAL | SRATIONAL | DOUBLE | LONG8 | SLONG8 | IFD8 => 8,
        }
    }
}
