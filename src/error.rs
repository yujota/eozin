use std::io;
use thiserror::Error;

use self::{DecodeError::*, EozinError::*};

#[cfg(any(feature = "wasm-node", feature = "wasm-web"))]
use wasm_bindgen::{JsError, JsValue};

/// Error type for this library
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum EozinError {
    /// An error occurs during decoding process.
    #[error("Decoding Error {0}")]
    Decoding(String),
    /// An error from IO operations
    #[error("IO Error")]
    IoError(#[from] io::Error),
    #[error("IO Error")]
    /// An error occurs when selected level is not found.
    LevelNotFound { selected: usize, num_level: usize },
    #[error("IO Error")]
    /// An error occurs when selected tile index is not found.
    TileNotFound { x: usize, y: usize, z: usize },
    /// An error occurs when an unexpected decoding process is encountered.
    #[error("Internal Error")]
    UnexpectedStep,
    /// An error occurs when trying to allocate extremly large buffer.
    #[error("Internal Error")]
    UnableToAllocateLargeSize,
    /// An error occurs when the input file cannot be handled by this library.
    #[error("Unkown Slide Format {0}")]
    UnknownFormat(&'static str),
    /// An error from JS operations
    #[error("JS Error {0}")]
    JsError(String),
    /// An error from JPEG 2000 operations
    #[error("JPEG 2000 Decoding Error {0}")]
    Jpeg2000DecodingError(String),
    /// An error from `image` crate
    #[error("Image Error {0}")]
    ImageError(String),
}

#[derive(Error, Debug)]
pub(crate) enum DecodeError {
    #[error("Tiff Error {0}")]
    DecodingTiff(crate::tiff_tools::error::TiffError),
    #[error("NDPI Error {0}")]
    NdpiJpegDecodingError(&'static str),
    #[error("ReadTile Error {0}")]
    ReadTileError(&'static str),
    #[error("Olympus Error {0}")]
    OlympusDecodingError(&'static str),
}

impl From<crate::tiff_tools::error::TiffError> for DecodeError {
    fn from(err: crate::tiff_tools::error::TiffError) -> DecodeError {
        DecodingTiff(err)
    }
}

impl From<crate::tiff_tools::error::TiffError> for EozinError {
    fn from(err: crate::tiff_tools::error::TiffError) -> EozinError {
        let e: DecodeError = err.into();
        Decoding(e.to_string())
    }
}

impl From<DecodeError> for EozinError {
    fn from(err: DecodeError) -> EozinError {
        Decoding(err.to_string())
    }
}

#[cfg(feature = "image")]
impl From<image::ImageError> for EozinError {
    fn from(err: image::ImageError) -> EozinError {
        ImageError(err.to_string())
    }
}

#[cfg(any(feature = "wasm-node", feature = "wasm-web"))]
impl From<JsValue> for EozinError {
    fn from(err: JsValue) -> EozinError {
        EozinError::JsError(format!("js error {:?}", err).to_string())
    }
}

#[cfg(any(feature = "wasm-node", feature = "wasm-web"))]
impl From<EozinError> for JsValue {
    fn from(err: EozinError) -> JsValue {
        JsError::new(&format!("js error {:?}", err)).into()
    }
}
