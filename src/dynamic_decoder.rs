use crate::format::{aperio, generic_tiff, ndpi, olympus, philips};
use crate::{
    base::{
        DecoderConstructor, EozinDecoderCore, LevelInfo, LevelInfoHandler, MapOutput,
        OrElseReadConsumer as OrElse, OutputMapper,
        ReadCommand::{self},
        ReadConsumer, ReadTileDecoder,
    },
    error::EozinError::{self, UnexpectedStep},
    tiff_tools::{TiffDecoder, data::Ifd, tags::CommonTag::*},
};

/// File format for digital pathology.
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SlideFormat {
    Aperio,
    HamamatsuNdpi,
    Philips,
    GenericTiff,
    OlympusETS,
}

impl std::fmt::Display for SlideFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SlideFormat::*;
        let t = match self {
            Aperio => "Aperio",
            HamamatsuNdpi => "Hamamatsu NDPI",
            Philips => "Philips",
            GenericTiff => "Generic TIFF",
            OlympusETS => "Olympus ETS",
        };
        write!(f, "{}", t)
    }
}

pub(crate) enum DynamicDecoder {
    Aperio(aperio::AperioDecoder),
    Ndpi(ndpi::NdpiDecoder),
    Philips(philips::PhilipsDecoder),
    Generic(generic_tiff::GenericTiffDecoder),
    Olympus(olympus::EtsDecoder),
}

impl DynamicDecoder {
    pub fn slide_format(&self) -> SlideFormat {
        use DynamicDecoder::*;
        match self {
            Aperio(_) => SlideFormat::Aperio,
            Ndpi(_) => SlideFormat::HamamatsuNdpi,
            Philips(_) => SlideFormat::Philips,
            Generic(_) => SlideFormat::GenericTiff,
            Olympus(_) => SlideFormat::OlympusETS,
        }
    }
}

pub(crate) type DynamicDecoderConstructor = OrElse<
    OrElse<MapOutput<TiffDecoder, FromTiffIfd>, MapOutput<ndpi::NdpiDecoderConstructor, FromNdpi>>,
    MapOutput<olympus::EtsDecoderConstructor, FromOlympus>,
>;

impl DecoderConstructor for DynamicDecoderConstructor {}

pub(crate) struct FromTiffIfd {}

impl OutputMapper for FromTiffIfd {
    type Input = Vec<Ifd>;
    type Output = DynamicDecoder;
    type ErrorKind = EozinError;
    fn map(input: Self::Input) -> Result<Self::Output, Self::ErrorKind> {
        if input.iter().any(is_ndpi) {
            Err(UnexpectedStep)
        } else if input.iter().any(is_aperio) {
            aperio::AperioDecoder::with_ifds(&input).map(DynamicDecoder::Aperio)
        } else if input.iter().any(is_philips) {
            philips::PhilipsDecoder::with_ifds(&input).map(DynamicDecoder::Philips)
        } else {
            generic_tiff::GenericTiffDecoder::with_ifds(&input).map(DynamicDecoder::Generic)
        }
    }
}

pub(crate) struct FromNdpi {}

impl OutputMapper for FromNdpi {
    type Input = ndpi::NdpiDecoder;
    type Output = DynamicDecoder;
    type ErrorKind = EozinError;
    fn map(input: Self::Input) -> Result<Self::Output, Self::ErrorKind> {
        Ok(DynamicDecoder::Ndpi(input))
    }
}

pub(crate) struct FromOlympus {}

impl OutputMapper for FromOlympus {
    type Input = olympus::EtsDecoder;
    type Output = DynamicDecoder;
    type ErrorKind = EozinError;
    fn map(input: Self::Input) -> Result<Self::Output, Self::ErrorKind> {
        Ok(DynamicDecoder::Olympus(input))
    }
}

fn is_aperio(ifd: &Ifd) -> bool {
    ifd.get(ImageDescription)
        .and_then(|d| d.expect_ascii())
        .map(|s| s.to_lowercase().contains("aperio"))
        .unwrap_or(false)
}

fn is_philips(ifd: &Ifd) -> bool {
    ifd.get(Software)
        .and_then(|d| d.expect_ascii())
        .map(|s| s.to_lowercase().contains("philips"))
        .unwrap_or(false)
}

fn is_ndpi(ifd: &Ifd) -> bool {
    ifd.get(Make)
        .and_then(|d| d.expect_ascii())
        .map(|s| s.to_lowercase().contains("hamamatsu"))
        .unwrap_or(false)
}

impl LevelInfoHandler for DynamicDecoder {
    fn get_level(&self, i: usize) -> Option<LevelInfo> {
        use DynamicDecoder::*;
        match self {
            Generic(d) => d.get_level(i),
            Aperio(d) => d.get_level(i),
            Ndpi(d) => d.get_level(i),
            Philips(d) => d.get_level(i),
            Olympus(d) => d.get_level(i),
        }
    }
    fn level_count(&self) -> usize {
        use DynamicDecoder::*;
        match self {
            Generic(d) => d.level_count(),
            Aperio(d) => d.level_count(),
            Ndpi(d) => d.level_count(),
            Philips(d) => d.level_count(),
            Olympus(d) => d.level_count(),
        }
    }
    fn marginal_tile_size(&self, lv: usize) -> Option<(u64, u64)> {
        use DynamicDecoder::*;
        match self {
            Generic(d) => d.marginal_tile_size(lv),
            Aperio(d) => d.marginal_tile_size(lv),
            Ndpi(d) => d.marginal_tile_size(lv),
            Philips(d) => d.marginal_tile_size(lv),
            Olympus(d) => d.marginal_tile_size(lv),
        }
    }
}

impl ReadTileDecoder for DynamicDecoder {
    type ReadTile = DynamicReadTile;
    type ReadTileInput = DynamicReadTileInput;
    fn read_tile(&self, lv: usize, x: usize, y: usize) -> Result<Self::ReadTileInput, EozinError> {
        match self {
            DynamicDecoder::Aperio(d) => d.read_tile(lv, x, y).map(DynamicReadTileInput::Aperio),
            DynamicDecoder::Ndpi(d) => d.read_tile(lv, x, y).map(DynamicReadTileInput::Ndpi),
            DynamicDecoder::Philips(d) => d.read_tile(lv, x, y).map(DynamicReadTileInput::Philips),
            DynamicDecoder::Generic(d) => d.read_tile(lv, x, y).map(DynamicReadTileInput::Generic),
            DynamicDecoder::Olympus(d) => d.read_tile(lv, x, y).map(DynamicReadTileInput::Olympus),
        }
    }
}

pub(crate) enum DynamicReadTile {
    Generic(generic_tiff::GenericReadTile),
    Aperio(aperio::AperioReadTile),
    Ndpi(ndpi::NdpiReadTile),
    Philips(philips::PhilipsReadTile),
    Olympus(olympus::EtsReadTile),
}

pub(crate) enum DynamicReadTileInput {
    Generic(generic_tiff::GenericReadTileInput),
    Aperio(aperio::AperioReadTileInput),
    Ndpi(ndpi::NdpiReadTileInput),
    Philips(philips::PhilipsReadTileInput),
    Olympus(olympus::EtsReadTileInput),
}

impl Default for DynamicReadTileInput {
    fn default() -> Self {
        DynamicReadTileInput::Generic(generic_tiff::GenericReadTileInput::default())
    }
}

impl ReadConsumer for DynamicReadTile {
    type Input = DynamicReadTileInput;
    type Output = Vec<u8>;
    type ErrorKind = EozinError;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand) {
        match input {
            DynamicReadTileInput::Generic(i) => {
                let (c, r) = generic_tiff::GenericReadTile::dispatch(i);
                (DynamicReadTile::Generic(c), r)
            }
            DynamicReadTileInput::Aperio(i) => {
                let (c, r) = aperio::AperioReadTile::dispatch(i);
                (DynamicReadTile::Aperio(c), r)
            }
            DynamicReadTileInput::Ndpi(i) => {
                let (c, r) = ndpi::NdpiReadTile::dispatch(i);
                (DynamicReadTile::Ndpi(c), r)
            }
            DynamicReadTileInput::Philips(i) => {
                let (c, r) = philips::PhilipsReadTile::dispatch(i);
                (DynamicReadTile::Philips(c), r)
            }
            DynamicReadTileInput::Olympus(i) => {
                let (c, r) = olympus::EtsReadTile::dispatch(i);
                (DynamicReadTile::Olympus(c), r)
            }
        }
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        use DynamicReadTile::*;
        match self {
            Generic(c) => c.receive(buf),
            Aperio(c) => c.receive(buf),
            Ndpi(c) => c.receive(buf),
            Philips(c) => c.receive(buf),
            Olympus(c) => c.receive(buf),
        }
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        use DynamicReadTile::*;
        match self {
            Generic(c) => c.step(buf),
            Aperio(c) => c.step(buf),
            Ndpi(c) => c.step(buf),
            Philips(c) => c.step(buf),
            Olympus(c) => c.step(buf),
        }
    }
}

impl EozinDecoderCore for DynamicDecoder {}
