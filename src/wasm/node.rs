//! Decoder works on nodejs/bun.
//!
//! This module requires the following crate feature to be activated: `wasm-node`.
use super::{Dimension, TileRange};
use crate::{
    base::{
        DecoderConstructor, EozinDecoderCore, MAX_ALLOCATION, MAX_LOOP_COUNT, ReadCommand::*,
        ReadConsumer, Tile,
    },
    dynamic_decoder as dynamic,
    error::EozinError::{self, UnableToAllocateLargeSize},
};
use js_sys::{Promise, Reflect, Uint8Array};
use std::marker::PhantomData;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

/// A decoder that dynamically detects file formats.
#[wasm_bindgen(js_name = Eozin)]
pub struct DynamicDecoder {
    decoder: EozinDecoder<dynamic::DynamicDecoder, dynamic::DynamicDecoderConstructor>,
}

#[wasm_bindgen(js_class = Eozin)]
impl DynamicDecoder {
    /// Constructs a new `DynamicDecoder` with a file path.
    #[wasm_bindgen(js_name = withPath)]
    pub async fn with_path(path: &str) -> Result<DynamicDecoder, EozinError> {
        let decoder = EozinDecoder::with_path(path).await?;
        Ok(DynamicDecoder { decoder })
    }

    /// Reads the tile at the specified level and coordinates.
    #[wasm_bindgen(js_name = readTile)]
    pub async fn read_tile(&self, lv: usize, x: usize, y: usize) -> Result<Tile, EozinError> {
        self.decoder.read_tile(lv, x, y).await
    }

    /// Returns the number of levels
    #[wasm_bindgen(js_name = levelCount, getter)]
    pub fn level_count(&self) -> usize {
        self.decoder.level_count()
    }

    /// Returns the dimensions (width, height) of the image at the highest
    /// resolution (level 0).
    #[wasm_bindgen(getter)]
    pub fn dimensions(&self) -> Dimension {
        self.decoder.dimensions()
    }

    /// Returns the dimensions for each level as a vector of (width, height)
    /// tuples, starting from level 0.
    #[wasm_bindgen(js_name = levelDimensions, getter)]
    pub fn level_dimensions(&self) -> Vec<Dimension> {
        self.decoder.level_dimensions()
    }

    /// Returns the nominal tile size (width, height) for each level.
    #[wasm_bindgen(js_name = levelTileSizes, getter)]
    pub fn level_tile_sizes(&self) -> Vec<Dimension> {
        self.decoder.level_tile_sizes()
    }

    /// Returns the grid range for each level as a vector of
    /// (horizontal_tile_count, vertical_tile_count) tuples.
    #[wasm_bindgen(js_name = levelTileRanges, getter)]
    pub fn level_tile_ranges(&self) -> Vec<TileRange> {
        self.decoder.level_tile_ranges()
    }

    /// Returns the format of slide.
    #[wasm_bindgen(js_name = slideFormat, getter)]
    pub fn slide_format(&self) -> String {
        self.decoder.decoder.slide_format().to_string()
    }
}

struct EozinDecoder<D, C>
where
    D: EozinDecoderCore,
    C: DecoderConstructor,
{
    decoder: D,
    file: FileHandler,
    _c: PhantomData<C>,
}

impl<D, C> EozinDecoder<D, C>
where
    D: EozinDecoderCore,
    C: DecoderConstructor<Output = D>,
{
    async fn with_path(path: &str) -> Result<EozinDecoder<D, C>, EozinError> {
        let file = FileHandler::new(path).await?;
        let decoder = file.excecute_read::<C>(()).await?;
        Ok(EozinDecoder {
            decoder,
            file,
            _c: PhantomData,
        })
    }
    async fn read_tile(&self, lv: usize, x: usize, y: usize) -> Result<Tile, EozinError> {
        let ri = self.decoder.read_tile(lv, x, y)?;
        let image_type = self
            .decoder
            .get_level(lv)
            .map(|l| l.image_type)
            .ok_or(EozinError::UnexpectedStep)?;
        let buf = self.file.excecute_read::<D::ReadTile>(ri).await?;
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
    fn level_tile_ranges(&self) -> Vec<TileRange> {
        self.decoder
            .level_tile_ranges()
            .into_iter()
            .map(|d| d.into())
            .collect()
    }
}

enum FileHandler {
    BunHandler(BunFile),
    NodeHandler(FileHandle),
}

impl FileHandler {
    async fn new(path: &str) -> Result<FileHandler, EozinError> {
        use FileHandler::*;
        if Self::is_runtime_bun().unwrap_or(false) {
            let file = Bun::file(path).unchecked_into::<BunFile>();
            Ok(BunHandler(file))
        } else {
            let file = open_node_file(path, "r")
                .await?
                .unchecked_into::<FileHandle>();
            Ok(NodeHandler(file))
        }
    }

    async fn excecute_read<C>(&self, input: C::Input) -> Result<C::Output, EozinError>
    where
        C: ReadConsumer<ErrorKind = EozinError>,
    {
        let (mut c, mut cmd) = C::dispatch(input);
        for _ in 0..MAX_LOOP_COUNT {
            match cmd {
                NoCmd => return c.receive(&[]),
                ReadBytes { offset, count } => {
                    let buf = self.read_bytes(offset, count).await?;
                    return c.receive(&buf);
                }
                ReadBytesStep { offset, count } => {
                    let buf = self.read_bytes(offset, count).await?;
                    cmd = c.step(&buf)?;
                }
            }
        }
        Err(UnableToAllocateLargeSize)
    }
    async fn read_bytes(&self, offset: u64, count: u64) -> Result<Vec<u8>, EozinError> {
        use FileHandler::*;
        if count > MAX_ALLOCATION {
            return Err(UnableToAllocateLargeSize);
        }
        match self {
            BunHandler(f) => read_bytes_bun(f, offset, count).await,
            NodeHandler(f) => read_bytes_node(f, offset, count).await,
        }
    }

    fn is_runtime_bun() -> Result<bool, JsValue> {
        let global = js_sys::global();
        let process = Reflect::get(&global, &JsValue::from_str("process"))?;
        let versions = Reflect::get(&process, &JsValue::from_str("versions"))?;
        let bun_version = Reflect::get(&versions, &JsValue::from_str("bun"))?;
        if bun_version.is_null_or_undefined() {
            Ok(false)
        } else {
            Ok(true)
        }
    }
}

#[wasm_bindgen]
extern "C" {
    type Bun;

    #[wasm_bindgen(static_method_of = Bun, js_name = file)]
    fn file(path: &str) -> JsValue;
}

#[wasm_bindgen]
extern "C" {
    type BunFile;

    #[wasm_bindgen(method)]
    fn bytes(this: &BunFile) -> Promise;

    #[wasm_bindgen(js_namespace = Bun, js_name = file)]
    fn bun_file(path: &str) -> BunFile;

    #[wasm_bindgen(method)]
    fn slice(this: &BunFile, start: f64, end: f64) -> BunFile;

    #[wasm_bindgen(method, getter)]
    fn size(this: &BunFile) -> f64;
}

async fn read_bytes_bun(file: &BunFile, offset: u64, count: u64) -> Result<Vec<u8>, EozinError> {
    let (start, end) = (offset as f64, (offset + count) as f64);
    let sliced = file.slice(start, end);
    let promise = sliced.bytes();
    let result = JsFuture::from(promise).await?;
    let uint8_array: Uint8Array = result.dyn_into()?;
    let buf = uint8_array.to_vec();
    Ok(buf)
}

#[wasm_bindgen(module = "node:fs/promises")]
extern "C" {
    #[wasm_bindgen(js_name = open)]
    fn open_node_file(path: &str, flags: &str) -> Promise;

    #[wasm_bindgen(js_name = FileHandle)]
    type FileHandle;

    #[wasm_bindgen(method)]
    fn read(
        this: &FileHandle,
        buffer: &Uint8Array,
        offset: f64,
        length: f64,
        position: f64,
    ) -> Promise;

    #[wasm_bindgen(method)]
    pub fn close(this: &FileHandle) -> Promise;
}

async fn read_bytes_node(
    file: &FileHandle,
    offset: u64,
    count: u64,
) -> Result<Vec<u8>, EozinError> {
    let c: u32 = count.try_into().map_err(|_| EozinError::UnexpectedStep)?;
    let buffer = Uint8Array::new_with_length(c);
    let read_promise = file.read(&buffer, 0.0, c as f64, offset as f64);
    let _ = JsFuture::from(read_promise).await?;
    Ok(buffer.to_vec())
}
