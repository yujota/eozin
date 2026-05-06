//! Decoder works with `R: Read + Seek`.
//!
//! This module requires the following crate feature to be activated: `std-io`.
use crate::error::EozinError::{self, *};

use crate::base::{
    DecoderConstructor, EozinDecoderCore, LevelInfo, ReadCommand::*, ReadConsumer, Tile,
    MAX_ALLOCATION, MAX_LOOP_COUNT
};
use crate::dynamic_decoder::{self, SlideFormat};
// use crate::format::philips;
// use crate::format::{aperio, generic_tiff, ndpi, olympus};
use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    marker::PhantomData,
    path::Path,
};

/// A decoder that dynamically detects file formats.
pub struct DynamicDecoder<R: Read + Seek> {
    decoder: DynamicDecoderBase<R>,
}

impl<R: Read + Seek> DynamicDecoder<R> {
    /// Constructs a new `DynamicDecoder` from an instance satisfying `Read + Seek`.
    ///
    /// When working with Olympus ETS files, specify the ETS file directly
    /// (e.g., `frame_t.ets`) rather than the VSI file.
    ///
    /// ```rust,no_run
    /// # use eozin::std_io::DynamicDecoder;
    /// let in_mem_buf = vec![0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08];
    /// let mut cur = std::io::Cursor::new(in_mem_buf);
    /// let mut decoder = DynamicDecoder::new(cur);
    /// ```
    pub fn new(r: R) -> Result<DynamicDecoder<R>, EozinError> {
        let decoder = DynamicDecoderBase::new(r)?;
        Ok(DynamicDecoder { decoder })
    }

    /// Reads the tile at the specified level and coordinates.
    /// All indices are 0-based.
    pub fn read_tile(&mut self, lv: usize, x: usize, y: usize) -> Result<Tile, EozinError> {
        self.decoder.read_tile(lv, x, y)
    }

    /// Reads the tile with the given index as `Vec<u8>`.
    /// This provides same functionality as follows;
    ///
    /// ```rust,no_run
    /// # use eozin::std_io::DynamicDecoder;
    /// # let mut decoder = DynamicDecoder::with_path("/some/slide.svs").unwrap();
    /// let buf = decoder.read_tile_as_bytes(0, 0, 0).unwrap();
    /// assert_eq!(buf, decoder.read_tile(0, 0, 0).unwrap().to_vec());
    /// ```
    pub fn read_tile_as_bytes(
        &mut self,
        lv: usize,
        x: usize,
        y: usize,
    ) -> Result<Vec<u8>, EozinError> {
        self.decoder.read_tile_as_bytes(lv, x, y)
    }

    /// Returns the number of levels
    pub fn level_count(&self) -> usize {
        self.decoder.level_count()
    }

    /// Returns the dimensions (width, height) of the image at the highest 
    /// resolution (level 0).
    pub fn dimensions(&self) -> (u64, u64) {
        self.decoder.dimensions()
    }

    /// Returns the dimensions for each level as a vector of (width, height) 
    /// tuples, starting from level 0.
    pub fn level_dimensions(&self) -> Vec<(u64, u64)> {
        self.decoder.level_dimensions()
    }

    /// Returns the nominal tile size (width, height) for each level.
    ///
    /// Note: While tiles within a level are usually of uniform size, 
    /// the rightmost and bottommost tiles may be smaller if the level's 
    /// total dimensions are not divisible by the tile size (common in NDPI files).
    /// Refer to the `width` and `height` properties of the returned [`Tile`] 
    /// for the actual dimensions of boundary tiles.
    ///
    /// ```rust,no_run
    /// # use eozin::{std_io::DynamicDecoder, Tile};
    /// let mut decoder = DynamicDecoder::with_path("/some/slide.ndpi").unwrap();
    /// let target_lv = decoder.level_count() - 1;
    /// let (lv_width, lv_height) = decoder.level_dimensions()[target_lv];
    /// let (tile_width, tile_height) = decoder.level_tile_sizes()[target_lv];
    ///
    /// let (num_horizontal, num_vertical) = decoder.level_tile_ranges()[target_lv];
    /// let right_most = decoder.read_tile(target_lv, num_horizontal - 1, 0).unwrap();
    ///
    /// // The width of the rightmost tile is the remainder of the level width 
    /// // divided by the standard tile width.
    /// assert_eq!(right_most.width as u64, (lv_width % tile_width));
    /// ```
    pub fn level_tile_sizes(&self) -> Vec<(u64, u64)> {
        self.decoder.level_tile_sizes()
    }

    /// Returns the grid range for each level as a vector of 
    /// (horizontal_tile_count, vertical_tile_count) tuples.
    pub fn level_tile_ranges(&self) -> Vec<(usize, usize)> {
        self.decoder.level_tile_ranges()
    }

    /// Returns the format of slide.
    pub fn slide_format(&self) -> SlideFormat {
        self.decoder.slide_format()
    }
}

impl DynamicDecoder<BufReader<File>> {
    /// Construct decoder from given file path.
    ///
    /// When working with OlympusETS files, specify the ETS file directly
    /// (often frame_t.ets) not the VSI file.
    ///
    /// ```rust,no_run
    /// # use eozin::std_io::DynamicDecoder;
    /// let mut decoder = DynamicDecoder::with_path("/some/slide.tiff");
    /// ```
    pub fn with_path<P: AsRef<Path>>(
        path: P,
    ) -> Result<DynamicDecoder<BufReader<File>>, EozinError> {
        let decoder = DynamicDecoderBase::with_path(path)?;
        Ok(DynamicDecoder { decoder })
    }
}

/*
pub struct AperioDecoder<R: Read + Seek> {
    decoder: AperioDecoderBase<R>,
}

impl<R: Read + Seek> AperioDecoder<R> {
    pub fn new(r: R) -> Result<AperioDecoder<R>, EozinError> {
        let decoder = AperioDecoderBase::new(r)?;
        Ok(AperioDecoder { decoder })
    }
    pub fn read_tile_as_bytes(
        &mut self,
        lv: usize,
        x: usize,
        y: usize,
    ) -> Result<Vec<u8>, EozinError> {
        self.decoder.read_tile_as_bytes(lv, x, y)
    }
    pub fn level_count(&self) -> usize {
        self.decoder.level_count()
    }
    pub fn dimensions(&self) -> (u64, u64) {
        self.decoder.dimensions()
    }
    pub fn level_dimensions(&self) -> Vec<(u64, u64)> {
        self.decoder.level_dimensions()
    }
    pub fn level_tile_sizes(&self) -> Vec<(u64, u64)> {
        self.decoder.level_tile_sizes()
    }
    pub fn level_tile_ranges(&self) -> Vec<(usize, usize)> {
        self.decoder.level_tile_ranges()
    }
}
*/

/*
pub struct NdpiDecoder<R: Read + Seek> {
    decoder: NdpiDecoderBase<R>,
}

impl<R: Read + Seek> NdpiDecoder<R> {
    pub fn new(r: R) -> Result<NdpiDecoder<R>, EozinError> {
        let decoder = NdpiDecoderBase::new(r)?;
        Ok(NdpiDecoder { decoder })
    }
    pub fn read_tile_as_bytes(
        &mut self,
        lv: usize,
        x: usize,
        y: usize,
    ) -> Result<Vec<u8>, EozinError> {
        self.decoder.read_tile_as_bytes(lv, x, y)
    }
    pub fn level_count(&self) -> usize {
        self.decoder.level_count()
    }
    pub fn dimensions(&self) -> (u64, u64) {
        self.decoder.dimensions()
    }
    pub fn level_dimensions(&self) -> Vec<(u64, u64)> {
        self.decoder.level_dimensions()
    }
    pub fn level_tile_sizes(&self) -> Vec<(u64, u64)> {
        self.decoder.level_tile_sizes()
    }
    pub fn level_tile_ranges(&self) -> Vec<(usize, usize)> {
        self.decoder.level_tile_ranges()
    }
}
*/

/*
pub struct OlympusDecoder<R: Read + Seek> {
    decoder: OlympusDecoderBase<R>,
}

impl<R: Read + Seek> OlympusDecoder<R> {
    pub fn new(r: R) -> Result<OlympusDecoder<R>, EozinError> {
        let decoder = OlympusDecoderBase::new(r)?;
        Ok(OlympusDecoder { decoder })
    }
    pub fn read_tile_as_bytes(
        &mut self,
        lv: usize,
        x: usize,
        y: usize,
    ) -> Result<Vec<u8>, EozinError> {
        self.decoder.read_tile_as_bytes(lv, x, y)
    }
    pub fn level_count(&self) -> usize {
        self.decoder.level_count()
    }
    pub fn dimensions(&self) -> (u64, u64) {
        self.decoder.dimensions()
    }
    pub fn level_dimensions(&self) -> Vec<(u64, u64)> {
        self.decoder.level_dimensions()
    }
    pub fn level_tile_sizes(&self) -> Vec<(u64, u64)> {
        self.decoder.level_tile_sizes()
    }
    pub fn level_tile_ranges(&self) -> Vec<(usize, usize)> {
        self.decoder.level_tile_ranges()
    }
}
*/

struct EozinDecoder<R, D, C>
where
    R: Read + Seek,
    D: EozinDecoderCore,
    C: DecoderConstructor,
{
    pub(crate) decoder: D,
    pub(crate) r: R,
    _c: PhantomData<C>,
}

#[allow(dead_code)]
impl<R, D, C> EozinDecoder<R, D, C>
where
    R: Read + Seek,
    D: EozinDecoderCore,
    C: DecoderConstructor<Output = D>,
{
    fn new(mut r: R) -> Result<EozinDecoder<R, D, C>, EozinError> {
        let decoder = excecute_read::<_, C>(&mut r, C::Input::default())?;
        Ok(EozinDecoder {
            decoder,
            r,
            _c: PhantomData,
        })
    }
    fn read_tile_as_bytes(&mut self, lv: usize, x: usize, y: usize) -> Result<Vec<u8>, EozinError> {
        let ri = self.decoder.read_tile(lv, x, y)?;
        let img = excecute_read::<_, D::ReadTile>(&mut self.r, ri)?;
        Ok(img)
    }
    fn read_tile(&mut self, lv: usize, x: usize, y: usize) -> Result<Tile, EozinError> {
        let buf = self.read_tile_as_bytes(lv, x, y)?;
        let image_type = self
            .decoder
            .get_level(lv)
            .map(|l| l.image_type)
            .ok_or(EozinError::UnexpectedStep)?;
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
    fn get_level(&self, i: usize) -> Option<LevelInfo> {
        self.decoder.get_level(i)
    }
    fn dimensions(&self) -> (u64, u64) {
        self.decoder.dimensions()
    }
    fn level_dimensions(&self) -> Vec<(u64, u64)> {
        self.decoder.level_dimensions()
    }
    fn level_tile_sizes(&self) -> Vec<(u64, u64)> {
        self.decoder.level_tile_sizes()
    }
    fn level_tile_ranges(&self) -> Vec<(usize, usize)> {
        self.decoder.level_tile_ranges()
    }
}

#[allow(dead_code)]
impl<D, C> EozinDecoder<BufReader<File>, D, C>
where
    D: EozinDecoderCore,
    C: DecoderConstructor<Output = D>,
{
    fn with_path<P: AsRef<Path>>(
        path: P,
    ) -> Result<EozinDecoder<BufReader<File>, D, C>, EozinError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::new(reader)
    }
}

type DynamicDecoderBase<R> =
    EozinDecoder<R, dynamic_decoder::DynamicDecoder, dynamic_decoder::DynamicDecoderConstructor>;

#[allow(dead_code)]
impl<R: Read + Seek> DynamicDecoderBase<R> {
    pub fn slide_format(&self) -> SlideFormat {
        self.decoder.slide_format()
    }

    /*
    pub fn expect_aperio(self) -> Option<AperioDecoderBase<R>> {
        let EozinDecoder { decoder, r, .. } = self;
        if let dynamic_decoder::DynamicDecoder::Aperio(decoder) = decoder {
            Some(EozinDecoder {
                decoder,
                r,
                _c: PhantomData,
            })
        } else {
            None
        }
    }

    pub fn expect_philips(self) -> Option<PhilipsDecoderBase<R>> {
        let EozinDecoder { decoder, r, .. } = self;
        if let dynamic_decoder::DynamicDecoder::Philips(decoder) = decoder {
            Some(EozinDecoder {
                decoder,
                r,
                _c: PhantomData,
            })
        } else {
            None
        }
    }
    */
}

/*
type AperioDecoderBase<R> =
    EozinDecoder<R, aperio::AperioDecoder, aperio::AperioDecoderConstructor>;

#[allow(dead_code)]
type PhilipsDecoderBase<R> =
    EozinDecoder<R, philips::PhilipsDecoder, philips::PhilipsDecoderConstructor>;

type NdpiDecoderBase<R> = EozinDecoder<R, ndpi::NdpiDecoder, ndpi::NdpiDecoderConstructor>;

type OlympusDecoderBase<R> = EozinDecoder<R, olympus::EtsDecoder, olympus::EtsDecoderConstructor>;

#[allow(dead_code)]
type GenericTiffDecoderBase<R> =
    EozinDecoder<R, generic_tiff::GenericTiffDecoder, generic_tiff::GenericTiffDecoderConstructor>;
*/

fn excecute_read<R, C>(mut r: &mut R, input: C::Input) -> Result<C::Output, EozinError>
where
    R: Read + Seek,
    C: ReadConsumer<ErrorKind = EozinError>,
{
    let (mut c, mut cmd) = C::dispatch(input);
    for _ in 0..MAX_LOOP_COUNT {
        match cmd {
            NoCmd => return c.receive(&[]),
            ReadBytes { offset, count } => {
                let buf = read_bytes(&mut r, offset, count)?;
                return c.receive(&buf);
            }
            ReadBytesStep { offset, count } => {
                let buf = read_bytes(&mut r, offset, count)?;
                cmd = c.step(&buf)?;
            }
        }
    }
    Err(UnableToAllocateLargeSize)
}

fn read_bytes<R: Read + Seek>(
    data: &mut R,
    offset: u64,
    count: u64,
) -> Result<Vec<u8>, EozinError> {
    if count > MAX_ALLOCATION {
        return Err(UnableToAllocateLargeSize);
    }
    let mut buffer = vec![0_u8; count as usize];
    data.seek(SeekFrom::Start(offset))?;
    let l = data.read(&mut buffer)?;
    if l == count as usize {
        Ok(buffer)
    } else {
        Err(UnexpectedStep)
    }
}
