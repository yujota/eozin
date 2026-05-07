use crate::error::EozinError;
use std::marker::PhantomData;

#[allow(dead_code)]
pub(crate) const MAX_ALLOCATION: u64 = 2 * 1024 * 1024 * 1024;
#[allow(dead_code)]
pub(crate) const MAX_LOOP_COUNT: usize = 1024 * 1024;

#[cfg(any(feature = "wasm-node", feature = "wasm-web"))]
use {js_sys::Uint8Array, wasm_bindgen::prelude::wasm_bindgen};

#[cfg(feature = "wasm-web")]
use {js_sys::Array, web_sys::Blob};

pub(crate) trait EozinDecoderCore: ReadTileDecoder + LevelInfoHandler {}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct LevelInfo {
    pub(crate) width: u64,
    pub(crate) height: u64,
    pub(crate) tile_width: u64,
    pub(crate) tile_height: u64,
    pub(crate) tile_range_x: usize,
    pub(crate) tile_range_y: usize,
    pub(crate) image_type: ImageType,
}

#[non_exhaustive]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ImageType {
    Jpeg,
    Jp2kRgb,
    Jp2kYCbCr,
    Bmp,
    Tiff,
}

/// Contains the encoded tile byte sequence, dimensions, and image format
/// information.
///
/// This struct implements `Deref<Target = [u8]>` and `AsRef<[u8]>`, allowing it to be used
/// directly as an encoded byte slice.
///
/// ```rust,no_run
/// # #[cfg(feature= "std-io")]
/// # {
/// # use eozin::{std_io::DynamicDecoder, Tile};
/// # let mut decoder = DynamicDecoder::with_path("/some/slide.svs").unwrap();
/// let tile: Tile = decoder.read_tile(0, 0, 0).unwrap();
/// std::fs::write("output.jpg", &tile).unwrap();
/// # }
/// ```
#[allow(dead_code)]
#[cfg_attr(any(feature = "wasm-web", feature = "wasm-node"), wasm_bindgen)]
#[derive(Debug, Clone)]
pub struct Tile {
    pub(crate) buf: Vec<u8>,
    pub(crate) image_type: ImageType,
    /// Width of the tile.
    pub width: u32,
    /// Height of the tile.
    pub height: u32,
}

#[allow(dead_code)]
impl Tile {
    /// Clones the internal encoded buffer.
    pub fn to_vec(&self) -> Vec<u8> {
        self.buf.to_vec()
    }
    /// Check if the tile is encoded in JPEG format.
    pub fn is_jpeg(&self) -> bool {
        matches!(self.image_type, ImageType::Jpeg)
    }
    /// Check if the tile is encoded in JPEG 2000 format.
    pub fn is_jpeg2000(&self) -> bool {
        matches!(self.image_type, ImageType::Jp2kYCbCr | ImageType::Jp2kRgb)
    }
    /// Check if the tile is encoded in TIFF format.
    pub fn is_tiff(&self) -> bool {
        matches!(self.image_type, ImageType::Tiff)
    }
    /// Check if the tile is encoded in BMP format.
    pub fn is_bmp(&self) -> bool {
        matches!(self.image_type, ImageType::Bmp)
    }

    /// Returns the `ImageFormat` from the `image` crate.
    ///
    /// Note: The current `image` crate does not support JPEG 2000.
    /// If the tile is encoded in JPEG 2000, this method returns `None`.
    ///
    /// This method requires the `image` feature.
    #[cfg(feature = "image")]
    pub fn image_format(&self) -> Option<image::ImageFormat> {
        match self.image_type {
            ImageType::Bmp => Some(image::ImageFormat::Bmp),
            ImageType::Tiff => Some(image::ImageFormat::Tiff),
            ImageType::Jpeg => Some(image::ImageFormat::Jpeg),
            ImageType::Jp2kRgb | ImageType::Jp2kYCbCr => None,
        }
    }

    /// Returns the encoded tile as a `DynamicImage` from the `image` crate.
    ///
    /// If the tile is encoded in JPEG 2000, it will be automatically
    /// converted to a format compatible with the `image` crate.
    ///
    /// This method requires the `image` feature.
    #[cfg(feature = "image")]
    #[allow(dead_code)]
    #[allow(unreachable_patterns)]
    pub fn to_image(&self) -> Result<image::DynamicImage, EozinError> {
        use ImageType::*;
        match self.image_type {
            Jpeg => Ok(image::load_from_memory_with_format(
                &self.buf,
                image::ImageFormat::Jpeg,
            )?),
            Tiff => Ok(image::load_from_memory_with_format(
                &self.buf,
                image::ImageFormat::Tiff,
            )?),
            Bmp => Ok(image::load_from_memory_with_format(
                &self.buf,
                image::ImageFormat::Bmp,
            )?),
            Jp2kYCbCr => {
                let new_buf = crate::jp2k::convert_jp2k_buf_to_image_buf(
                    &self.buf,
                    image::ImageFormat::Bmp,
                    true,
                )?;
                Ok(image::load_from_memory_with_format(
                    &new_buf,
                    image::ImageFormat::Bmp,
                )?)
            }
            Jp2kRgb => {
                let new_buf = crate::jp2k::convert_jp2k_buf_to_image_buf(
                    &self.buf,
                    image::ImageFormat::Bmp,
                    false,
                )?;
                Ok(image::load_from_memory_with_format(
                    &new_buf,
                    image::ImageFormat::Bmp,
                )?)
            }
            _ => Err(EozinError::UnknownFormat("Cannot handle this format")),
        }
    }
}

#[cfg_attr(any(feature = "wasm-web", feature = "wasm-node"), wasm_bindgen)]
impl Tile {
    /// Returns the MIME type of the encoded tile image.
    #[cfg_attr(any(feature = "wasm-web", feature = "wasm-node"), wasm_bindgen(js_name = mimeType, getter))]
    pub fn mime_type(&self) -> String {
        let s = match self.image_type {
            ImageType::Jp2kRgb | ImageType::Jp2kYCbCr => "image/jp2",
            ImageType::Bmp => "image/bmp",
            ImageType::Jpeg => "image/jpeg",
            ImageType::Tiff => "image/tiff",
        };
        s.to_string()
    }
}

#[allow(dead_code)]
#[cfg(any(feature = "wasm-web", feature = "wasm-node"))]
#[allow(unreachable_patterns)]
#[wasm_bindgen]
impl Tile {
    /// Returns the raw encoded tile buffer as a `Uint8Array`.
    /// No format conversion is performed.
    ///
    /// This method requires the `wasm-node` or `wasm-web` feature.
    #[wasm_bindgen(js_name = toUint8Array)]
    pub fn to_uint8array(&self) -> Result<Uint8Array, EozinError> {
        let array = Uint8Array::new_from_slice(&self.buf);
        Ok(array)
    }

    /// Returns the tile as a `Blob`.
    ///
    /// Formats not natively supported by browsers (such as TIFF or JPEG 2000)
    /// are automatically converted to PNG.
    ///
    /// Note: The `Tile::mime_type()` method returns the original image format,
    /// which may differ from the MIME type of the `Blob` returned here.
    ///
    /// This method requires the `wasm-web` feature.
    ///
    /// ```rust,no_run
    /// # #[cfg(feature= "wasm-web")]
    /// # {
    /// # use eozin::wasm::web::DynamicDecoder;
    /// # use web_sys::Blob;
    /// # async fn main() {
    /// # let decoder = DynamicDecoder::with_blob(Blob::new().unwrap()).await.unwrap();
    /// # let tile = decoder.read_tile(0, 0, 0).await.unwrap();
    /// assert!(tile.is_jpeg2000());
    /// let blob = tile.to_blob().unwrap();
    /// assert_eq!(&blob.type_(), "image/png");
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "wasm-web")]
    #[wasm_bindgen(js_name = toBlob)]
    pub fn to_blob(&self) -> Result<Blob, EozinError> {
        let (arr, mime) = match self.image_type {
            ImageType::Jpeg => (Uint8Array::new_from_slice(&self.buf), "image/jpeg"),
            ImageType::Bmp => (Uint8Array::new_from_slice(&self.buf), "image/bmp"),
            ImageType::Tiff => {
                let img = image::load_from_memory_with_format(&self.buf, image::ImageFormat::Tiff)?;
                let mut converted = Vec::new();
                img.write_to(
                    &mut std::io::Cursor::new(&mut converted),
                    image::ImageFormat::Png,
                )?;
                (Uint8Array::new_from_slice(&converted), "image/png")
            }
            ImageType::Jp2kYCbCr => {
                let converted = crate::jp2k::convert_jp2k_buf_to_image_buf(
                    &self.buf,
                    image::ImageFormat::Png,
                    true,
                )?;
                (Uint8Array::new_from_slice(&converted), "image/png")
            }
            ImageType::Jp2kRgb => {
                let converted = crate::jp2k::convert_jp2k_buf_to_image_buf(
                    &self.buf,
                    image::ImageFormat::Png,
                    false,
                )?;
                (Uint8Array::new_from_slice(&converted), "image/png")
            }
        };
        let options = web_sys::BlobPropertyBag::new();
        options.set_type(mime);
        Ok(web_sys::Blob::new_with_u8_array_sequence_and_options(
            &Array::of1(&arr),
            &options,
        )?)
    }
}

impl std::ops::Deref for Tile {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

impl AsRef<[u8]> for Tile {
    fn as_ref(&self) -> &[u8] {
        &self.buf
    }
}

pub(crate) trait ReadTileDecoder {
    type ReadTileInput: Default;
    type ReadTile: ReadConsumer<Input = Self::ReadTileInput, Output = Vec<u8>, ErrorKind = EozinError>;
    fn read_tile(&self, lv: usize, x: usize, y: usize) -> Result<Self::ReadTileInput, EozinError>;
}

#[allow(dead_code)]
pub(crate) trait LevelInfoHandler {
    fn level_count(&self) -> usize;
    fn get_level(&self, i: usize) -> Option<LevelInfo>;
    fn marginal_tile_size(&self, lv: usize) -> Option<(u64, u64)>;
    fn dimensions(&self) -> (u64, u64) {
        let lv = self.get_level(0).unwrap();
        (lv.width, lv.height)
    }
    fn level_dimensions(&self) -> Vec<(u64, u64)> {
        (0..self.level_count())
            .map(|lv| self.get_level(lv).unwrap())
            .map(|lv| (lv.width, lv.height))
            .collect()
    }
    fn level_tile_sizes(&self) -> Vec<(u64, u64)> {
        (0..self.level_count())
            .map(|lv| self.get_level(lv).unwrap())
            .map(|lv| (lv.tile_width, lv.tile_height))
            .collect()
    }
    fn level_tile_ranges(&self) -> Vec<(usize, usize)> {
        (0..self.level_count())
            .map(|lv| self.get_level(lv).unwrap())
            .map(|lv| (lv.tile_range_x, lv.tile_range_y))
            .collect()
    }
    fn level_marginal_tile_sizes(&self) -> Vec<Option<(u64, u64)>> {
        (0..self.level_count())
            .map(|lv| self.marginal_tile_size(lv))
            .collect()
    }
    fn tile_size(&self, lv: usize, x: usize, y: usize) -> Option<(u64, u64)> {
        let ts = self.level_tile_sizes();
        let (tw, th) = ts.get(lv)?;
        if let Some((x_marginal, y_marginal)) = self.marginal_tile_size(lv) {
            let rs = self.level_tile_ranges();
            let (x_m, y_m) = rs.get(lv)?;
            if x < x_m - 1 && y < y_m - 1 {
                Some((*tw, *th))
            } else if x == x_m - 1 && y < y_m - 1 {
                Some((x_marginal, *th))
            } else if x < x_m - 1 && y == y_m - 1 {
                Some((*tw, y_marginal))
            } else if x == x_m - 1 && y == y_m - 1 {
                Some((x_marginal, y_marginal))
            } else {
                Some((*tw, *th))
            }
        } else {
            Some((*tw, *th))
        }
    }
}

pub(crate) trait DecoderConstructor:
    ReadConsumer<Input = (), ErrorKind = EozinError, Output: EozinDecoderCore>
{
}

pub(crate) enum ReadCommand {
    NoCmd,
    ReadBytes { offset: u64, count: u64 },
    ReadBytesStep { offset: u64, count: u64 },
}

pub(crate) trait ReadConsumer {
    type Input: Default;
    type Output;
    type ErrorKind;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand)
    where
        Self: Sized;
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind>;
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind>;
    fn mock() -> Self
    where
        Self: Sized,
    {
        Self::dispatch(Self::Input::default()).0
    }
}

pub(crate) enum ConcatReadConsumer<L, R>
where
    L: ReadConsumer + Sized,
    R: ReadConsumer<Input = L::Output, ErrorKind = L::ErrorKind> + Sized,
{
    ProcLeft(L),
    RecLeft(L),
    ProcRight(R),
}

impl<L, R> ReadConsumer for ConcatReadConsumer<L, R>
where
    L: ReadConsumer,
    R: ReadConsumer<Input = L::Output, ErrorKind = L::ErrorKind>,
{
    type Input = L::Input;
    type Output = R::Output;
    type ErrorKind = L::ErrorKind;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand) {
        use ConcatReadConsumer::*;
        use ReadCommand::*;
        let (left, r) = L::dispatch(input);
        match r {
            NoCmd => {
                panic!("Unkown todo");
            }
            ReadBytes { offset, count } => (RecLeft(left), ReadBytesStep { offset, count }),
            ReadBytesStep { offset, count } => (ProcLeft(left), ReadBytesStep { offset, count }),
        }
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        use ConcatReadConsumer::*;
        match self {
            ProcRight(right) => right.receive(buf),
            _ => {
                panic!("Invalid")
            }
        }
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        use ConcatReadConsumer::*;
        use ReadCommand::*;
        match self {
            ProcLeft(left) => {
                let r = left.step(buf)?;
                match r {
                    NoCmd => {
                        let mut left_ = L::mock();
                        std::mem::swap(&mut left_, left);
                        let right_input = left_.receive(&[])?;
                        let (right, r) = R::dispatch(right_input);
                        *self = ProcRight(right);
                        Ok(r)
                    }
                    ReadBytes { offset, count } => {
                        let mut left_ = L::mock();
                        std::mem::swap(&mut left_, left);
                        *self = RecLeft(left_);
                        Ok(ReadBytesStep { offset, count })
                    }
                    ReadBytesStep { .. } => Ok(r),
                }
            }
            RecLeft(left) => {
                let mut left_ = L::mock();
                std::mem::swap(&mut left_, left);
                let right_input = left_.receive(&[])?;
                let (right, r) = R::dispatch(right_input);
                *self = ProcRight(right);
                Ok(r)
            }
            ProcRight(right) => right.step(buf),
        }
    }
}

pub(crate) enum OrElseReadConsumer<L, R>
where
    L: ReadConsumer<Input: Clone> + Sized,
    R: ReadConsumer<Input = L::Input, Output = L::Output, ErrorKind = L::ErrorKind> + Sized,
{
    ProcLeft((L, L::Input)),
    ProcRight(R),
    Finished(L::Output),
}

impl<L, R> ReadConsumer for OrElseReadConsumer<L, R>
where
    L: ReadConsumer<Input: Clone> + Sized,
    R: ReadConsumer<Input = L::Input, Output = L::Output, ErrorKind = L::ErrorKind> + Sized,
{
    type Input = L::Input;
    type Output = R::Output;
    type ErrorKind = L::ErrorKind;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand) {
        use OrElseReadConsumer::*;
        use ReadCommand::*;
        let (left, r) = L::dispatch(input.clone());
        match r {
            NoCmd => {
                panic!("Unkown todo");
            }
            ReadBytes { offset, count } => {
                (ProcLeft((left, input)), ReadBytesStep { offset, count })
            }
            ReadBytesStep { offset, count } => {
                (ProcLeft((left, input)), ReadBytesStep { offset, count })
            }
        }
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        use OrElseReadConsumer::*;
        match self {
            Finished(o) => Ok(o),
            ProcRight(right) => right.receive(buf),
            _ => {
                panic!("Invalid")
            }
        }
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        use OrElseReadConsumer::*;
        use ReadCommand::*;
        match self {
            ProcLeft((left, inp)) => match left.step(buf) {
                Ok(NoCmd) => {
                    let mut left_ = L::mock();
                    std::mem::swap(&mut left_, left);
                    if let Ok(l_out) = left_.receive(&[]) {
                        *self = Finished(l_out);
                        Ok(NoCmd)
                    } else {
                        let (right, r) = R::dispatch(inp.clone());
                        *self = ProcRight(right);
                        Ok(r)
                    }
                }
                Ok(ReadBytes { offset, count }) => Ok(ReadBytesStep { offset, count }),
                Ok(ReadBytesStep { offset, count }) => Ok(ReadBytesStep { offset, count }),
                Err(_) => {
                    let (right, r) = R::dispatch(inp.clone());
                    *self = ProcRight(right);
                    Ok(r)
                }
            },
            ProcRight(right) => right.step(buf),
            Finished(_) => Ok(NoCmd),
        }
    }
}

pub(crate) trait OutputMapper {
    type Input;
    type Output;
    type ErrorKind;
    fn map(input: Self::Input) -> Result<Self::Output, Self::ErrorKind>;
}

pub(crate) struct MapOutput<C, M>
where
    C: ReadConsumer + Sized,
    M: OutputMapper<Input = C::Output, ErrorKind = C::ErrorKind>,
{
    c: C,
    _m: PhantomData<M>,
}

impl<C, M> ReadConsumer for MapOutput<C, M>
where
    C: ReadConsumer + Sized,
    M: OutputMapper<Input = C::Output, ErrorKind = C::ErrorKind>,
{
    type Input = C::Input;
    type Output = M::Output;
    type ErrorKind = C::ErrorKind;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand)
    where
        Self: Sized,
    {
        let (c, r) = C::dispatch(input);
        (MapOutput { c, _m: PhantomData }, r)
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        let MapOutput { c, .. } = self;
        let out = c.receive(buf)?;
        M::map(out)
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        self.c.step(buf)
    }
}
