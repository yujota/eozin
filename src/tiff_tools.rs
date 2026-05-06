pub(crate) mod bytes_parser;
pub(crate) mod data;
pub(crate) mod decode;
pub(crate) mod error;
pub(crate) mod tags;

pub use data::Ifd;
pub use tags::CommonTag::{self};

use crate::base::{ReadCommand, ReadConsumer};
use crate::error::EozinError;

pub(crate) struct TiffDecoder {
    decoder: decode::Decoder,
}

impl ReadConsumer for TiffDecoder {
    type Input = ();
    type Output = Vec<Ifd>;
    type ErrorKind = EozinError;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand) {
        let (decoder, r) = decode::Decoder::dispatch(input);
        (TiffDecoder { decoder }, r)
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        let TiffDecoder { decoder } = self;
        Ok(decoder.receive(buf)?)
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        Ok(self.decoder.step(buf)?)
    }
}
