pub(crate) mod data;
pub(crate) mod parser;
pub(crate) mod tag;

pub(crate) use data::{Data, Tiff};
pub(crate) use parser::{ParseTiffError, Parser, Size};
