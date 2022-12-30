#[derive(Debug, PartialEq, Clone)]
pub(crate) enum TiffError {
    FormatError { msg: String },
}

pub(in crate::tiff) fn format_error(msg: &str) -> TiffError {
    TiffError::FormatError {
        msg: msg.to_string(),
    }
}
