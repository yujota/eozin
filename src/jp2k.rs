use crate::error::EozinError;

pub(crate) fn convert_jp2k_buf_to_image_buf(
    buf: &[u8],
    output_format: image::ImageFormat,
    ycbcr_to_rgb: bool,
) -> Result<Vec<u8>, EozinError> {
    let img = hayro_jpeg2000::Image::new(&buf, &hayro_jpeg2000::DecodeSettings::default())?;
    let (width, height) = (img.width(), img.height());
    let bitmap = img.decode()?;
    let bitmap = if ycbcr_to_rgb {
        convert_ycbcr_to_rgb(&bitmap, width as usize, height as usize, img.has_alpha()).ok_or(
            EozinError::Jpeg2000DecodingError("Failed to convert YCbCr to RGB".to_string()),
        )?
    } else {
        bitmap
    };
    if img.has_alpha() {
        let img = image::RgbaImage::from_raw(width, height, bitmap).ok_or(
            EozinError::Jpeg2000DecodingError("Failed to convert JPEG2000".to_string()),
        )?;
        let mut cursor = std::io::Cursor::new(Vec::new());
        img.write_to(&mut cursor, output_format).map_err(|e| {
            EozinError::Jpeg2000DecodingError(
                format!("Failed to convert JPEG2000 {}", e).to_string(),
            )
        })?;
        Ok(cursor.into_inner())
    } else {
        let img = image::RgbImage::from_raw(width, height, bitmap).ok_or(
            EozinError::Jpeg2000DecodingError("Failed to convert JPEG2000".to_string()),
        )?;
        let mut cursor = std::io::Cursor::new(Vec::new());
        img.write_to(&mut cursor, output_format).map_err(|e| {
            EozinError::Jpeg2000DecodingError(
                format!("Failed to convert JPEG2000 {}", e).to_string(),
            )
        })?;
        Ok(cursor.into_inner())
    }
}

fn convert_ycbcr_to_rgb(
    buf: &[u8],
    width: usize,
    height: usize,
    has_alpha: bool,
) -> Option<Vec<u8>> {
    let count = width.checked_mul(height)?;
    let p = if has_alpha { 4 } else { 3 };
    let length = count.checked_mul(p)?;
    if buf.len() < length {
        return None;
    }
    let mut out = buf[..length].to_vec();

    for i in 0..count {
        let y = f32::from(buf[i * p]);
        let cb = f32::from(buf[i * p + 1]);
        let cr = f32::from(buf[i * p + 2]);

        let r = (y + 1.402 * (cr - 128.0)).clamp(0.0, 255.0) as u8;
        let g = (y - 0.344136 * (cb - 128.0) - 0.714136 * (cr - 128.0)).clamp(0.0, 255.0) as u8;
        let b = (y + 1.772 * (cb - 128.0)).clamp(0.0, 255.0) as u8;

        out[i * p] = r;
        out[i * p + 1] = g;
        out[i * p + 2] = b;
    }
    Some(out)
}

impl From<hayro_jpeg2000::DecodeError> for EozinError {
    fn from(err: hayro_jpeg2000::DecodeError) -> EozinError {
        EozinError::Jpeg2000DecodingError(err.to_string())
    }
}
