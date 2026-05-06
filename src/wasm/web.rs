//! Decoder works on web browser.
//!
//! This module requires the following crate feature to be activated: `wasm-web`.
use crate::error::EozinError;

use super::{Dimension, TileRange};
use crate::base::{DecoderConstructor, EozinDecoderCore, ReadCommand::*, ReadConsumer, Tile};
use crate::dynamic_decoder as dynamic;
use js_sys;
use std::marker::PhantomData;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::{Blob, File};

/// A decoder that dynamically detects file formats.
#[wasm_bindgen(js_name = Eozin)]
pub struct DynamicDecoder {
    decoder: DynamicDecoderBase,
}

#[wasm_bindgen(js_class = Eozin)]
impl DynamicDecoder {
    /// Constructs a new `DynamicDecoder` with a `Blob`.
    #[wasm_bindgen]
    pub async fn with_blob(blob: Blob) -> Result<DynamicDecoder, EozinError> {
        let decoder = DynamicDecoderBase::with_blob(blob).await?;
        Ok(DynamicDecoder { decoder })
    }

    /// Constructs a new `DynamicDecoder` with a `File`.
    #[wasm_bindgen]
    pub async fn with_file(file: File) -> Result<DynamicDecoder, EozinError> {
        let decoder = DynamicDecoderBase::with_file(file).await?;
        Ok(DynamicDecoder { decoder })
    }

    /// Reads the tile at the specified level and coordinates.
    #[wasm_bindgen]
    pub async fn read_tile(&self, lv: usize, x: usize, y: usize) -> Result<Tile, EozinError> {
        self.decoder.read_tile(lv, x, y).await
    }

    /// Returns the number of levels
    #[wasm_bindgen]
    pub fn level_count(&self) -> usize {
        self.decoder.level_count()
    }

    /// Returns the dimensions (width, height) of the image at the highest 
    /// resolution (level 0).
    #[wasm_bindgen]
    pub fn dimensions(&self) -> Dimension {
        self.decoder.dimensions()
    }

    /// Returns the dimensions for each level as a vector of (width, height) 
    /// tuples, starting from level 0.
    #[wasm_bindgen]
    pub fn level_dimensions(&self) -> Vec<Dimension> {
        self.decoder.level_dimensions()
    }

    /// Returns the nominal tile size (width, height) for each level.
    #[wasm_bindgen]
    pub fn level_tile_sizes(&self) -> Vec<Dimension> {
        self.decoder.level_tile_sizes()
    }

    /// Returns the grid range for each level as a vector of 
    /// (horizontal_tile_count, vertical_tile_count) tuples.
    #[wasm_bindgen]
    pub fn level_tile_ranges(&self) -> Vec<TileRange> {
        self.decoder.level_tile_ranges()
    }

    /// Returns the format of slide.
    #[wasm_bindgen]
    pub fn slide_format(&self) -> String {
        self.decoder.decoder.slide_format().to_string()
    }
}

struct EozinDecoder<D, C>
where
    D: EozinDecoderCore,
    C: DecoderConstructor,
{
    pub(crate) decoder: D,
    blob: Blob,
    _c: PhantomData<C>,
}

impl<D, C> EozinDecoder<D, C>
where
    D: EozinDecoderCore,
    C: DecoderConstructor<Output = D>,
{
    async fn with_blob(blob: Blob) -> Result<EozinDecoder<D, C>, EozinError> {
        let decoder = excecute_read::<C>(&blob, ()).await?;
        Ok(EozinDecoder {
            decoder,
            blob,
            _c: PhantomData,
        })
    }
    async fn with_file(file: File) -> Result<EozinDecoder<D, C>, EozinError> {
        let blob = Blob::from(file);
        Self::with_blob(blob).await
    }
    async fn read_tile(&self, lv: usize, x: usize, y: usize) -> Result<Tile, EozinError> {
        let ri = self.decoder.read_tile(lv, x, y)?;
        let image_type = self
            .decoder
            .get_level(lv)
            .map(|l| l.image_type)
            .ok_or(EozinError::UnexpectedStep)?;
        let buf = excecute_read::<D::ReadTile>(&self.blob, ri).await?;
        let (width, height) = self
            .decoder
            .tile_size(lv, x, y)
            .and_then(|(w, h)| w.try_into().ok().zip(h.try_into().ok()))
            .ok_or(EozinError::UnexpectedStep)?;
        Ok(Tile {
            buf,
            image_type,
            width,
            height,
        })
    }
    fn level_count(&self) -> usize {
        self.decoder.level_count()
    }
    fn dimensions(&self) -> Dimension {
        self.decoder.dimensions().into()
    }
    fn level_dimensions(&self) -> Vec<Dimension> {
        self.decoder
            .level_dimensions()
            .into_iter()
            .map(|d| d.into())
            .collect()
    }
    fn level_tile_sizes(&self) -> Vec<Dimension> {
        self.decoder
            .level_tile_sizes()
            .into_iter()
            .map(|d| d.into())
            .collect()
    }
    /*
    fn level_marginal_tile_sizes(&self) -> Vec<Dimension> {
        let ms = self.decoder.level_marginal_tile_sizes();
        let ts = self.level_tile_sizes();
        ms.into_iter()
            .zip(ts)
            .map(|(m, t)| {
                if let Some((mw, mh)) = m {
                    Dimension {
                        width: mw as u32,
                        height: mh as u32,
                    }
                } else {
                    t
                }
            })
            .collect()
    }
    */
    fn level_tile_ranges(&self) -> Vec<TileRange> {
        self.decoder
            .level_tile_ranges()
            .into_iter()
            .map(|d| d.into())
            .collect()
    }
}

type DynamicDecoderBase = EozinDecoder<dynamic::DynamicDecoder, dynamic::DynamicDecoderConstructor>;

async fn excecute_read<C>(blob: &Blob, input: C::Input) -> Result<C::Output, EozinError>
where
    C: ReadConsumer<ErrorKind = EozinError>,
{
    let (mut c, mut cmd) = C::dispatch(input);
    loop {
        match cmd {
            NoCmd => return c.receive(&[]),
            ReadBytes { offset, count } => {
                let buf = read_bytes(blob, offset, count).await?;
                return c.receive(&buf);
            }
            ReadBytesStep { offset, count } => {
                let buf = read_bytes(blob, offset, count).await?;
                cmd = c.step(&buf)?;
            }
        }
    }
}

async fn read_bytes(blob: &Blob, offset: u64, count: u64) -> Result<Vec<u8>, EozinError> {
    let (start, end) = (offset as f64, (offset + count) as f64);
    let sliced_blob =
        blob.slice_with_f64_and_f64_and_content_type(start, end, "application/octet-stream")?;
    let buffer = wasm_bindgen_futures::JsFuture::from(sliced_blob.array_buffer()).await?;
    let array = js_sys::Uint8Array::new(&buffer);
    Ok(array.to_vec())
}
