#[derive(Debug)]
pub struct LittleEndian {}

#[derive(Debug)]
pub struct BigEndian {}

#[allow(dead_code)]
pub trait BytesParser {
    fn ascii(buf: &[u8]) -> Option<String>
    where
        Self: Sized,
    {
        buf.iter().try_fold(String::new(), |mut acc, x| {
            match std::char::from_u32(*x as u32) {
                Some(c) => {
                    acc.push(c);
                    Some(acc)
                }
                _ => None,
            }
        })
    }
    fn rational(buf: &[u8]) -> Option<(u32, u32)>
    where
        Self: Sized,
    {
        if buf.len() >= 8 {
            match (Self::u32(&buf[0..4]), Self::u32(&buf[4..8])) {
                (Some(numer), Some(denom)) => Some((numer, denom)),
                _ => None,
            }
        } else {
            None
        }
    }
    fn rational_vec(buf: &[u8]) -> Option<Vec<(u32, u32)>>
    where
        Self: Sized,
    {
        if buf.len() < 8 {
            return None;
        }
        buf.chunks_exact(8).map(|x| Self::rational(x)).collect()
    }
    fn u8(buf: &[u8]) -> Option<u8>
    where
        Self: Sized,
    {
        buf.first().copied()
    }
    fn u8_vec(buf: &[u8]) -> Option<Vec<u8>>
    where
        Self: Sized,
    {
        Some(buf.to_vec())
    }
    fn u16(buf: &[u8]) -> Option<u16>
    where
        Self: Sized;
    fn u16_vec(buf: &[u8]) -> Option<Vec<u16>>
    where
        Self: Sized,
    {
        if !buf.len().is_multiple_of(2) {
            return None;
        }
        buf.chunks_exact(2).map(|x| Self::u16(x)).collect()
    }
    fn u32(buf: &[u8]) -> Option<u32>
    where
        Self: Sized;
    fn u32_vec(buf: &[u8]) -> Option<Vec<u32>>
    where
        Self: Sized,
    {
        if !buf.len().is_multiple_of(4) {
            return None;
        }
        buf.chunks_exact(4).map(|x| Self::u32(x)).collect()
    }
    fn u64(buf: &[u8]) -> Option<u64>
    where
        Self: Sized;
    fn u64_vec(buf: &[u8]) -> Option<Vec<u64>>
    where
        Self: Sized,
    {
        if !buf.len().is_multiple_of(8) {
            return None;
        }
        buf.chunks_exact(8).map(|x| Self::u64(x)).collect()
    }
    fn i8(buf: &[u8]) -> Option<i8>
    where
        Self: Sized,
    {
        buf.first().copied().map(|x| x as i8)
    }
    fn i8_vec(buf: &[u8]) -> Option<Vec<i8>>
    where
        Self: Sized,
    {
        Some(buf.iter().map(|x| *x as i8).collect())
    }
    fn i16(buf: &[u8]) -> Option<i16>
    where
        Self: Sized;
    fn i16_vec(buf: &[u8]) -> Option<Vec<i16>>
    where
        Self: Sized,
    {
        if !buf.len().is_multiple_of(2) {
            return None;
        }
        buf.chunks_exact(2).map(|x| Self::i16(x)).collect()
    }
    fn i32(buf: &[u8]) -> Option<i32>
    where
        Self: Sized;
    fn i32_vec(buf: &[u8]) -> Option<Vec<i32>>
    where
        Self: Sized,
    {
        if !buf.len().is_multiple_of(4) {
            return None;
        }
        buf.chunks_exact(4).map(|x| Self::i32(x)).collect()
    }
    fn i64(buf: &[u8]) -> Option<i64>
    where
        Self: Sized;
    fn i64_vec(buf: &[u8]) -> Option<Vec<i64>>
    where
        Self: Sized,
    {
        if !buf.len().is_multiple_of(8) {
            return None;
        }
        buf.chunks_exact(8).map(|x| Self::i64(x)).collect()
    }
    fn f32(buf: &[u8]) -> Option<f32>
    where
        Self: Sized;
    fn f32_vec(buf: &[u8]) -> Option<Vec<f32>>
    where
        Self: Sized,
    {
        if !buf.len().is_multiple_of(4) {
            return None;
        }
        buf.chunks_exact(4).map(|x| Self::f32(x)).collect()
    }
    fn f64(buf: &[u8]) -> Option<f64>
    where
        Self: Sized;
    fn f64_vec(buf: &[u8]) -> Option<Vec<f64>>
    where
        Self: Sized,
    {
        if !buf.len().is_multiple_of(8) {
            return None;
        }
        buf.chunks_exact(8).map(|x| Self::f64(x)).collect()
    }
}

impl BytesParser for LittleEndian {
    fn u16(buf: &[u8]) -> Option<u16> {
        if buf.len() >= 2 {
            Some(u16::from_le_bytes([buf[0], buf[1]]))
        } else {
            None
        }
    }
    fn u32(buf: &[u8]) -> Option<u32> {
        if buf.len() >= 4 {
            Some(u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]))
        } else {
            None
        }
    }
    fn u64(buf: &[u8]) -> Option<u64> {
        if buf.len() >= 8 {
            Some(u64::from_le_bytes([
                buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
            ]))
        } else {
            None
        }
    }
    fn i16(buf: &[u8]) -> Option<i16> {
        if buf.len() >= 2 {
            Some(i16::from_le_bytes([buf[0], buf[1]]))
        } else {
            None
        }
    }
    fn i32(buf: &[u8]) -> Option<i32> {
        if buf.len() >= 4 {
            Some(i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]))
        } else {
            None
        }
    }
    fn i64(buf: &[u8]) -> Option<i64> {
        if buf.len() >= 8 {
            Some(i64::from_le_bytes([
                buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
            ]))
        } else {
            None
        }
    }
    fn f32(buf: &[u8]) -> Option<f32> {
        if buf.len() >= 4 {
            Some(f32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]))
        } else {
            None
        }
    }
    fn f64(buf: &[u8]) -> Option<f64> {
        if buf.len() >= 8 {
            Some(f64::from_le_bytes([
                buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
            ]))
        } else {
            None
        }
    }
}

impl BytesParser for BigEndian {
    fn u16(buf: &[u8]) -> Option<u16> {
        if buf.len() >= 2 {
            Some(u16::from_be_bytes([buf[0], buf[1]]))
        } else {
            None
        }
    }
    fn u32(buf: &[u8]) -> Option<u32> {
        if buf.len() >= 4 {
            Some(u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]))
        } else {
            None
        }
    }
    fn u64(buf: &[u8]) -> Option<u64> {
        if buf.len() >= 8 {
            Some(u64::from_be_bytes([
                buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
            ]))
        } else {
            None
        }
    }
    fn i16(buf: &[u8]) -> Option<i16> {
        if buf.len() >= 2 {
            Some(i16::from_be_bytes([buf[0], buf[1]]))
        } else {
            None
        }
    }
    fn i32(buf: &[u8]) -> Option<i32> {
        if buf.len() >= 4 {
            Some(i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]))
        } else {
            None
        }
    }
    fn i64(buf: &[u8]) -> Option<i64> {
        if buf.len() >= 8 {
            Some(i64::from_be_bytes([
                buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
            ]))
        } else {
            None
        }
    }
    fn f32(buf: &[u8]) -> Option<f32> {
        if buf.len() >= 4 {
            Some(f32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]))
        } else {
            None
        }
    }
    fn f64(buf: &[u8]) -> Option<f64> {
        if buf.len() >= 8 {
            Some(f64::from_be_bytes([
                buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
            ]))
        } else {
            None
        }
    }
}
