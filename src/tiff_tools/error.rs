use super::data::DataType;
use std::{error, fmt};

#[derive(Debug, Clone)]
pub enum TiffError {
    DecodeDataError {
        dt: DataType,
        count: usize,
        msg: &'static str,
    },
    DecodeCountError {
        msg: &'static str,
    },
    DecodeEntryError {
        msg: &'static str,
    },
    UnsupportedDataType {
        dt: DataType,
        msg: &'static str,
    },
    InvalidDecodingProcess(&'static str),
    ParseHeaderError(&'static str),
    ParseIfdError(&'static str),
}

impl fmt::Display for TiffError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use TiffError::*;
        match self {
            DecodeDataError { dt, count, msg } => write!(
                f,
                "Tiff Decode Data Error: target dt {}, count {}, {}",
                dt, count, msg
            )
            .unwrap(),
            UnsupportedDataType { dt, msg } => {
                write!(f, "Error During parsing {}: {}", dt, msg).unwrap()
            }
            DecodeCountError { msg } => write!(f, "Error During decodoing count {}", msg).unwrap(),
            DecodeEntryError { msg } => write!(f, "Error During decodoing entry {}", msg).unwrap(),
            InvalidDecodingProcess(msg) => {
                write!(f, "Error During decodoing entry {}", msg).unwrap()
            }
            _ => write!(f, "Error During decodoing entry").unwrap(),
        }
        Ok(())
    }
}

impl error::Error for TiffError {}
