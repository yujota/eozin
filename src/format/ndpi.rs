use super::ndpi_big::NdpiBigIfdDecoder;
use crate::base::{
    ConcatReadConsumer, DecoderConstructor, EozinDecoderCore, ImageType, LevelInfo,
    LevelInfoHandler, OrElseReadConsumer,
    ReadCommand::{self, *},
    ReadConsumer, ReadTileDecoder,
};
use crate::error::{
    DecodeError::*,
    EozinError::{self, *},
};
use crate::jpeg_tools::{JpegParser, Segment::*};
use crate::tiff_tools::{Ifd, TiffDecoder, CommonTag::*};
use std::collections::VecDeque;

pub(crate) type NdpiDecoderConstructor =
    ConcatReadConsumer<OrElseReadConsumer<TiffDecoder, NdpiBigIfdDecoder>, IfdToNdpiDecoder>;

impl DecoderConstructor for NdpiDecoderConstructor {}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct NdpiDecoder {
    levels: Vec<Level>,
    assocs: Vec<AssociatedLevel>,
}

impl EozinDecoderCore for NdpiDecoder {}

#[allow(private_interfaces)]
pub(crate) enum IfdToNdpiDecoder {
    Proc {
        target: DecodingLevel,
        remaining: VecDeque<DecodingLevel>,
        post: Vec<DecodedLevel>,
        assocs: Vec<AssociatedLevel>,
    },
    Final {
        target: DecodingLevel,
        post: Vec<DecodedLevel>,
        assocs: Vec<AssociatedLevel>,
    },
    Finished {
        post: Vec<DecodedLevel>,
        assocs: Vec<AssociatedLevel>,
    },
}

#[derive(Debug)]
struct DecodingLevel {
    pub strip_offset: u64,
    pub strip_count: u64,
    pub jpeg_header_count: u64,
    pub width: u64,
    pub height: u64,
    pub restart_marker_offset: Vec<u64>,
    pub restart_marker_count: Vec<u64>,
}

#[derive(Debug)]
struct DecodedLevel {
    pub strip_offset: u64,
    pub strip_count: u64,
    pub restart_marker_offset: Vec<u64>,
    pub restart_marker_count: Vec<u64>,
    pub dri: usize, // Restart maker interval
    pub jpeg_tables: Vec<u8>,
    pub sof_offset: usize,
    pub width: u64,
    pub height: u64,
}

#[allow(dead_code)]
#[derive(Debug)]
struct Level {
    pub info: LevelInfo,
    pub strip_offset: u64,
    pub strip_count: u64,
    pub restart_marker_offset: Vec<u64>,
    pub restart_marker_count: Vec<u64>,
    pub jpeg_tables: Vec<u8>,
    pub sof_offset: usize,
    pub interval: TileInterval, // Restart maker interval
}

#[allow(dead_code)]
#[derive(Debug)]
struct TileInterval {
    pub dri: usize,                     // Restart maker interval
    pub merge_unit: usize,              // max_dri / dri
    pub horizontal_unit: usize,         // level_width / (dri * 8)
    pub vertical_unit: usize,           // level_height / 8
    pub in_tile_vertical_unit: usize,   // = max_dri = tile_length / 8
    pub in_tile_horizontal_unit: usize, // merge_unit
    pub right_unit_rem: usize,          // num reminder unit for right marginal tile. it should be
    // less than max_dri
    pub bottom_unit_rem: usize, // num reminder unit for bottom marginal tile. it should be
    // less than (tile_length / 8)
    pub margin_right: u16,  // max y index for reminder(bottom level)
    pub margin_bottom: u16, // max y index for reminder(bottom level)
}

#[allow(dead_code)]
#[derive(Debug)]
struct AssociatedLevel {
    pub width: u64,
    pub height: u64,
    pub strip_offset: u64,
    pub strip_count: u64,
}

#[allow(dead_code)]
impl NdpiDecoder {
    fn from_levels(
        levels: Vec<DecodedLevel>,
        assocs: Vec<AssociatedLevel>,
    ) -> Result<Self, EozinError> {
        let max_dri = levels.iter().map(|l| l.dri).max().ok_or(UnexpectedStep)?;
        println!("Max DRI {}", max_dri);
        let levels = levels
            .into_iter()
            .map(|l| Self::to_level(l, max_dri))
            .collect::<Result<Vec<Level>, _>>()?;
        Ok(NdpiDecoder { levels, assocs })
    }

    fn to_level(lv: DecodedLevel, max_dri: usize) -> Result<Level, EozinError> {
        if !max_dri.is_multiple_of(lv.dri) {
            return Err(NdpiJpegDecodingError(
                "Lv0 DRI is not divisible by other levels's DRI",
            ))?;
        }
        let dri = lv.dri;
        let tile_length = max_dri.checked_mul(8).ok_or(UnexpectedStep)?;
        let unit_length = dri.checked_mul(8).ok_or(UnexpectedStep)?;
        let tl_u16: u16 = tile_length.try_into().map_err(|_| UnexpectedStep)?;
        if !lv.width.is_multiple_of(unit_length as u64) {
            return Err(NdpiJpegDecodingError(
                "Width of Level is not divisibe by DRI size x 8",
            ))?;
        }
        let mut jpeg_tables = lv.jpeg_tables.clone();
        update_img_shape(&mut jpeg_tables, lv.sof_offset, Some(tl_u16), Some(tl_u16));
        let interval =
            TileInterval::new(dri, max_dri, lv.width, lv.height).ok_or(UnexpectedStep)?;
        let tile_range_x = lv
            .width
            .try_into()
            .ok()
            .and_then(|x: usize| x.checked_div(tile_length))
            .map(|x| {
                if interval.right_unit_rem != 0 {
                    x + 1
                } else {
                    x
                }
            })
            .ok_or(UnexpectedStep)?;
        let tile_range_y = lv
            .height
            .try_into()
            .ok()
            .and_then(|x: usize| x.checked_div(tile_length))
            .map(|x| {
                if interval.bottom_unit_rem != 0 {
                    x + 1
                } else {
                    x
                }
            })
            .ok_or(UnexpectedStep)?;
        let info = LevelInfo {
            width: lv.width,
            height: lv.height,
            tile_width: tile_length as u64,
            tile_height: tile_length as u64,
            tile_range_x,
            tile_range_y,
            image_type: ImageType::Jpeg,
        };
        Ok(Level {
            info,
            strip_offset: lv.strip_offset,
            strip_count: lv.strip_count,
            restart_marker_count: lv.restart_marker_count,
            restart_marker_offset: lv.restart_marker_offset,
            interval,
            jpeg_tables,
            sof_offset: lv.sof_offset,
        })
    }
}

impl ReadConsumer for IfdToNdpiDecoder {
    type Input = Vec<Ifd>;
    type Output = NdpiDecoder;
    type ErrorKind = EozinError;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand) {
        // println!("IFD NUM {:?}", input.len());
        use IfdToNdpiDecoder::*;
        let mut remaining: VecDeque<DecodingLevel> = VecDeque::new();
        let mut assocs: Vec<AssociatedLevel> = Vec::new();
        for ifd in input {
            if let Some(d) = DecodingLevel::from_ifd(&ifd) {
                remaining.push_back(d);
            } else if let Some(a) = AssociatedLevel::from_ifd(&ifd) {
                assocs.push(a);
            }
        }
        if remaining.is_empty() {
            (
                Finished {
                    post: vec![],
                    assocs,
                },
                NoCmd,
            )
        } else {
            let target = remaining.pop_front().unwrap();
            let r = ReadBytesStep {
                offset: target.strip_offset,
                count: target.jpeg_header_count,
            };
            let post = vec![];
            (
                Proc {
                    remaining,
                    target,
                    post,
                    assocs,
                },
                r,
            )
        }
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        use EozinError::*;
        use IfdToNdpiDecoder::*;
        match self {
            Final {
                target,
                mut post,
                assocs,
            } => {
                match target.to_level(buf) {
                    Ok(lv) => {
                        post.push(lv);
                    }
                    Err(e) => {
                        println!("During IFDtoDecoder {}", e);
                    }
                };
                NdpiDecoder::from_levels(post, assocs)
            }
            Finished { post, assocs } => NdpiDecoder::from_levels(post, assocs),
            _ => Err(UnexpectedStep),
        }
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        use IfdToNdpiDecoder::*;
        match self {
            Final {
                target,
                post,
                assocs,
            } => {
                match target.to_level(buf) {
                    Ok(lv) => {
                        post.push(lv);
                    }
                    Err(e) => {
                        println!("During IFDtoDecoder {}", e);
                    }
                };
                let mut levels = vec![];
                std::mem::swap(&mut levels, post);
                let mut assocs_ = vec![];
                std::mem::swap(&mut assocs_, assocs);
                *self = Finished {
                    post: levels,
                    assocs: assocs_,
                };
                Ok(NoCmd)
            }
            Finished { .. } => Ok(NoCmd),
            Proc {
                target,
                remaining,
                post,
                assocs,
            } => {
                match target.to_level(buf) {
                    Ok(lv) => {
                        post.push(lv);
                    }
                    Err(e) => {
                        println!("During IFDtoDecoder {}", e);
                    }
                };
                if remaining.is_empty() {
                    let mut levels = vec![];
                    std::mem::swap(&mut levels, post);
                    let mut assocs_ = vec![];
                    std::mem::swap(&mut assocs_, assocs);
                    *self = Finished {
                        post: levels,
                        assocs: assocs_,
                    };
                    Ok(NoCmd)
                } else {
                    let next = remaining.pop_front().unwrap();
                    let offset = next.strip_offset;
                    let count = next.jpeg_header_count;
                    if remaining.is_empty() {
                        let mut post_ = vec![];
                        std::mem::swap(&mut post_, post);
                        let mut assocs_ = vec![];
                        std::mem::swap(&mut assocs_, assocs);
                        *self = Final {
                            target: next,
                            post: post_,
                            assocs: assocs_,
                        };
                        Ok(ReadBytes { offset, count })
                    } else {
                        *target = next;
                        Ok(ReadBytesStep { offset, count })
                    }
                }
            }
        }
    }
}

impl LevelInfoHandler for NdpiDecoder {
    fn get_level(&self, i: usize) -> Option<LevelInfo> {
        self.levels.get(i).map(|l| l.info)
    }
    fn level_count(&self) -> usize {
        self.levels.len()
    }
    fn marginal_tile_size(&self, lv: usize) -> Option<(u64, u64)> {
        let lv = self.levels.get(lv)?;
        if lv.interval.margin_bottom == 0 && lv.interval.margin_right == 0 {
            None
        } else {
            Some((
                lv.interval.margin_right as u64,
                lv.interval.margin_bottom as u64,
            ))
        }
    }
}

impl ReadTileDecoder for NdpiDecoder {
    type ReadTile = NdpiReadTile;
    type ReadTileInput = NdpiReadTileInput;
    fn read_tile(&self, lv: usize, x: usize, y: usize) -> Result<NdpiReadTileInput, EozinError> {
        let lv = self.levels.get(lv).ok_or(LevelNotFound {
            selected: lv,
            num_level: self.levels.len(),
        })?;
        if lv.is_out_of_range(x, y) {
            return Err(TileNotFound { x, y, z: 0 });
        }
        let mut strips = Vec::new();
        let mut jpeg_tables = lv.jpeg_tables.clone();
        let start = lv.interval.in_tile_vertical_unit * lv.interval.horizontal_unit * y
            + x * lv.interval.merge_unit;
        if lv.is_not_marginal(x, y) {
            for y_i in 0..lv.interval.in_tile_vertical_unit {
                let i = start + y_i * lv.interval.horizontal_unit;
                let ofs = lv.restart_marker_offset[i] + lv.strip_offset;
                let cs = lv.restart_marker_count[i..i + lv.interval.merge_unit].to_vec();
                let c: u64 = cs.iter().sum();
                strips.push(ReadTileStrip {
                    offset: ofs,
                    count: c,
                    dri_marker_counts: cs,
                });
            }
        } else if lv.is_right_marginal(x, y) {
            for y_i in 0..lv.interval.in_tile_vertical_unit {
                let i = start + y_i * lv.interval.horizontal_unit;
                let ofs = lv.restart_marker_offset[i] + lv.strip_offset;
                let cs = lv.restart_marker_count[i..i + lv.interval.right_unit_rem].to_vec();
                let c: u64 = cs.iter().sum();
                strips.push(ReadTileStrip {
                    offset: ofs,
                    count: c,
                    dri_marker_counts: cs,
                });
            }
            update_img_shape(
                &mut jpeg_tables,
                lv.sof_offset,
                Some(lv.interval.margin_right),
                None,
            );
        } else if lv.is_bottom_marginal(x, y) {
            for y_i in 0..lv.interval.bottom_unit_rem {
                let i = start + y_i * lv.interval.horizontal_unit;
                let ofs = lv.restart_marker_offset[i] + lv.strip_offset;
                let cs = lv.restart_marker_count[i..i + lv.interval.merge_unit].to_vec();
                let c: u64 = cs.iter().sum();
                strips.push(ReadTileStrip {
                    offset: ofs,
                    count: c,
                    dri_marker_counts: cs,
                });
            }
            update_img_shape(
                &mut jpeg_tables,
                lv.sof_offset,
                None,
                Some(lv.interval.margin_bottom),
            );
        } else if lv.is_bottom_right(x, y) {
            for y_i in 0..lv.interval.bottom_unit_rem {
                let i = start + y_i * lv.interval.horizontal_unit;
                let ofs = lv.restart_marker_offset[i] + lv.strip_offset;
                let cs = lv.restart_marker_count[i..i + lv.interval.right_unit_rem].to_vec();
                let c: u64 = cs.iter().sum();
                strips.push(ReadTileStrip {
                    offset: ofs,
                    count: c,
                    dri_marker_counts: cs,
                });
            }
            update_img_shape(
                &mut jpeg_tables,
                lv.sof_offset,
                Some(lv.interval.margin_right),
                Some(lv.interval.margin_bottom),
            );
        } else {
            return Err(TileNotFound { x, y, z: 0 });
        };
        Ok(NdpiReadTileInput {
            jpeg_tables,
            strips,
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct NdpiReadTileInput {
    pub jpeg_tables: Vec<u8>,
    pub strips: Vec<ReadTileStrip>,
}

#[derive(Debug, Default)]
pub(crate) struct ReadTileStrip {
    offset: u64,
    count: u64,
    dri_marker_counts: Vec<u64>,
}

#[derive(Debug)]
pub(crate) struct NdpiReadTile {
    pub jpeg_tables: Vec<u8>,
    pub strips: VecDeque<ReadTileStrip>,
    pub target: ReadTileStrip,
    pub dri_count: usize,
}

impl ReadConsumer for NdpiReadTile {
    type Input = NdpiReadTileInput;
    type Output = Vec<u8>;
    type ErrorKind = EozinError;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand) {
        let NdpiReadTileInput {
            jpeg_tables,
            strips,
        } = input;
        let mut strips: VecDeque<_> = strips.into();
        let target = strips.pop_front().unwrap();
        let (offset, count) = (target.offset, target.count);
        let r = if strips.is_empty() {
            ReadBytes { offset, count }
        } else {
            ReadBytesStep { offset, count }
        };
        (
            NdpiReadTile {
                jpeg_tables,
                strips,
                target,
                dri_count: 0,
            },
            r,
        )
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        let mut r = self;
        if !buf.is_empty() {
            r.update(buf)?;
        };
        let NdpiReadTile {
            mut jpeg_tables, ..
        } = r;
        jpeg_tables.extend_from_slice(&[0xFF, 0xD9]);
        Ok(jpeg_tables)
    }
    fn step(&mut self, buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        self.update(buf)?;
        if let Some(target) = self.strips.pop_front() {
            self.target = target;
            let (offset, count) = (self.target.offset, self.target.count);
            if self.strips.is_empty() {
                Ok(ReadBytes { offset, count })
            } else {
                Ok(ReadBytesStep { offset, count })
            }
        } else {
            Ok(NoCmd)
        }
    }
}

impl NdpiReadTile {
    fn update(&mut self, buf: &[u8]) -> Result<(), EozinError> {
        let mut buf = buf.to_vec();
        let mut acc: usize = 0;
        for c in self.target.dri_marker_counts.iter() {
            let dri_m: u8 = self
                .dri_count
                .checked_rem(8)
                .and_then(|x| x.try_into().ok())
                .and_then(|x: u8| x.checked_add(0xD0))
                .ok_or(UnexpectedStep)?;
            acc = (*c)
                .try_into()
                .ok()
                .and_then(|x: usize| acc.checked_add(x))
                .ok_or(UnexpectedStep)?;
            // println!("Acc {}, {}", acc, buf.len());
            let v = buf
                .get_mut(acc - 1)
                .ok_or(ReadTileError("unable to reach"))?;
            *v = dri_m;
            self.dri_count = self.dri_count.checked_add(1).ok_or(ReadTileError("hoge"))?;
        }
        self.jpeg_tables.extend_from_slice(&buf);
        Ok(())
    }
}

#[allow(dead_code)]
impl NdpiDecoder {
    pub(crate) fn associated_image(
        &self,
        lv: usize,
    ) -> Result<NdpiReadAssociatedImageInput, EozinError> {
        let lv = self.assocs.get(lv).ok_or(LevelNotFound {
            selected: lv,
            num_level: self.assocs.len(),
        })?;
        Ok(NdpiReadAssociatedImageInput {
            offset: lv.strip_offset,
            count: lv.strip_count,
        })
    }
    pub(crate) fn associated_level_count(&self) -> usize {
        self.assocs.len()
    }
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct NdpiReadAssociatedImageInput {
    pub offset: u64,
    pub count: u64,
}

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct NdpiReadAssociatedImage {}

impl ReadConsumer for NdpiReadAssociatedImage {
    type Input = NdpiReadAssociatedImageInput;
    type Output = Vec<u8>;
    type ErrorKind = EozinError;

    fn dispatch(input: Self::Input) -> (Self, ReadCommand) {
        let NdpiReadAssociatedImageInput { offset, count } = input;
        let r = ReadBytes { offset, count };
        (NdpiReadAssociatedImage {}, r)
    }
    fn receive(self, buf: &[u8]) -> Result<Self::Output, Self::ErrorKind> {
        Ok(buf.to_vec())
    }
    fn step(&mut self, _buf: &[u8]) -> Result<ReadCommand, Self::ErrorKind> {
        Err(UnexpectedStep)
    }
}

impl DecodingLevel {
    fn from_ifd(ifd: &Ifd) -> Option<Self> {
        use NdpiTag::*;
        let width = ifd.get(ImageWidth).and_then(|d| d.to_u64())?;
        let height = ifd.get(ImageLength).and_then(|d| d.to_u64())?;
        let strip_offset = ifd.get(StripOffsets).and_then(|d| d.to_u64())?;
        let strip_count = ifd.get(StripByteCounts).and_then(|d| d.to_u64())?;
        let restart_marker_offset: Vec<u64> = match (
            ifd.get(RestartMarkerLowerBytes)
                .and_then(|d| d.expect_u32vec()),
            ifd.get(RestartMarkerHigherBytes)
                .and_then(|d| d.expect_u32vec()),
        ) {
            (Some(l), Some(h)) => Some(
                l.iter()
                    .zip(h.iter())
                    .map(|(x, y)| {
                        // println!("l{} h{}", x, y);
                        let xs = x.to_le_bytes();
                        let ys = y.to_le_bytes();
                        u64::from_le_bytes([xs[0], xs[1], xs[2], xs[3], ys[0], ys[1], ys[2], ys[3]])
                    })
                    .collect(),
            ),
            (Some(l), None) => Some(l.iter().map(|x| *x as u64).collect()),
            _ => return None,
        }?;
        let restart_marker_count =
            Self::calc_count_from_offset(&restart_marker_offset, strip_count)?;
        let jpeg_header_count = *restart_marker_offset.first()?;
        Some(DecodingLevel {
            width,
            height,
            strip_count,
            strip_offset,
            restart_marker_count,
            restart_marker_offset,
            jpeg_header_count,
        })
    }

    fn to_level(&self, buf: &[u8]) -> Result<DecodedLevel, EozinError> {
        if self.jpeg_header_count != buf.len() as u64 {
            return Err(NdpiJpegDecodingError("JPEG Header is not match"))?;
        }
        let segments = JpegParser::parse(buf);
        let mut restart_interval = None;
        let mut start_frame: Option<(usize, u64, u64)> = None;
        for s in segments {
            match s.seg {
                StartOfFrame0 { width, height, .. } => {
                    start_frame = Some((s.offset, width as u64, height as u64))
                }
                RestartInterval(i) => restart_interval = Some(i),
                _ => continue,
            }
        }
        let (ofs, _width, _height) = start_frame.ok_or(NdpiJpegDecodingError("SOF not found"))?;
        let restart_interval = restart_interval.ok_or(NdpiJpegDecodingError("DRI not found"))?;
        let jpeg_tables = buf.to_vec();
        Ok(DecodedLevel {
            strip_offset: self.strip_offset,
            strip_count: self.strip_count,
            restart_marker_offset: self.restart_marker_offset.clone(),
            restart_marker_count: self.restart_marker_count.clone(),
            dri: restart_interval,
            jpeg_tables,
            sof_offset: ofs,
            width: self.width,
            height: self.height,
        })
    }
    fn calc_count_from_offset(ofs: &[u64], total_count: u64) -> Option<Vec<u64>> {
        let mut vs = ofs.to_vec();
        let mut counts = vec![];
        vs.push(total_count);
        let mut acc = 0;
        for (i, v) in vs.into_iter().enumerate() {
            if i == 0 {
                acc = v;
            } else if acc >= v {
                return None;
            } else {
                counts.push(v - acc);
                acc = v;
            }
        }
        Some(counts)
    }
}

impl Level {
    fn is_out_of_range(&self, x: usize, y: usize) -> bool {
        if x > self.info.tile_range_x - 1 || y > self.info.tile_range_y - 1 {
            true
        } else {
            false
        }
    }
    fn is_not_marginal(&self, x: usize, y: usize) -> bool {
        (x < self.info.tile_range_x - 1 && y < self.info.tile_range_y - 1)
            || (x == self.info.tile_range_x - 1
                && self.interval.right_unit_rem == 0
                && y < self.info.tile_range_y - 1)
            || (y == self.info.tile_range_y - 1
                && self.interval.bottom_unit_rem == 0
                && x < self.info.tile_range_x - 1)
            || (self.interval.right_unit_rem == 0 && self.interval.bottom_unit_rem == 0)
    }
    fn is_bottom_marginal(&self, x: usize, y: usize) -> bool {
        y == self.info.tile_range_y - 1
            && self.interval.bottom_unit_rem != 0
            && (x < self.info.tile_range_x - 1
                || (x == self.info.tile_range_x - 1 && self.interval.right_unit_rem == 0))
    }
    fn is_right_marginal(&self, x: usize, y: usize) -> bool {
        x == self.info.tile_range_x - 1
            && self.interval.right_unit_rem != 0
            && (y < self.info.tile_range_y - 1
                || (y == self.info.tile_range_y - 1 && self.interval.bottom_unit_rem == 0))
    }
    fn is_bottom_right(&self, x: usize, y: usize) -> bool {
        y == self.info.tile_range_y - 1
            && self.interval.bottom_unit_rem != 0
            && x == self.info.tile_range_x - 1
            && self.interval.right_unit_rem != 0
    }
}

impl TileInterval {
    fn new(dri: usize, max_dri: usize, width: u64, height: u64) -> Option<Self> {
        let merge_unit = max_dri.checked_div(dri)?;
        let unit_length = dri.checked_mul(8)?;
        let horizontal_unit = width
            .try_into()
            .ok()
            .and_then(|i: usize| i.checked_div(unit_length))?;
        let vertical_unit = height
            .try_into()
            .ok()
            .and_then(|i: usize| i.checked_div(8))?;
        let right_unit_rem = horizontal_unit.checked_rem(merge_unit)?;
        let bottom_unit_rem = vertical_unit.checked_rem(max_dri)?;
        let margin_right: u16 = right_unit_rem
            .checked_mul(unit_length)
            .and_then(|i| i.try_into().ok())?;
        let margin_bottom: u16 = bottom_unit_rem
            .checked_mul(8)
            .and_then(|i| i.try_into().ok())?;
        Some(TileInterval {
            dri,
            merge_unit,
            horizontal_unit,
            vertical_unit,
            in_tile_vertical_unit: max_dri,
            in_tile_horizontal_unit: merge_unit,
            right_unit_rem,
            bottom_unit_rem,
            margin_right,
            margin_bottom,
        })
    }
}

impl AssociatedLevel {
    fn from_ifd(ifd: &Ifd) -> Option<AssociatedLevel> {
        let width = ifd.get(ImageWidth).and_then(|d| d.to_u64())?;
        let height = ifd.get(ImageLength).and_then(|d| d.to_u64())?;
        let strip_offset = ifd.get(StripOffsets).and_then(|d| d.to_u64())?;
        let strip_count = ifd.get(StripByteCounts).and_then(|d| d.to_u64())?;
        let compression = ifd.get(Compression).and_then(|d| d.to_u64())?;
        if compression != 7 {
            // println!("only accepting jpeg(7) but given cmp {}", compression);
            return None;
        }
        // let photo_inter = ifd.get(PhotometricInterpretation).and_then(|d| d.to_u64());
        // println!("Photo inter {:?}", photo_inter);
        Some(AssociatedLevel {
            width,
            height,
            strip_count,
            strip_offset,
        })
    }
}

fn update_img_shape(jpeg_tables: &mut [u8], ofs: usize, w: Option<u16>, h: Option<u16>) {
    if let Some(w) = w {
        jpeg_tables[ofs + 4 + 1 + 2] = w.to_be_bytes()[0];
        jpeg_tables[ofs + 4 + 1 + 2 + 1] = w.to_be_bytes()[1];
    };
    if let Some(h) = h {
        jpeg_tables[ofs + 4 + 1] = h.to_be_bytes()[0];
        jpeg_tables[ofs + 4 + 1 + 1] = h.to_be_bytes()[1];
    };
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Copy, Clone, Eq)]
pub(crate) enum NdpiTag {
    RestartMarkerLowerBytes,
    RestartMarkerHigherBytes,
    SourceLens,             // 65421 SourceLens
    XOffsetFromSlideCenter, // 65422 	XOffsetFromSlideCentre
    YOffsetFromSlideCenter, // 65423 	YOffsetFromSlideCentre
}

impl From<NdpiTag> for u16 {
    fn from(tag: NdpiTag) -> u16 {
        use NdpiTag::*;
        match tag {
            RestartMarkerLowerBytes => 65426,
            RestartMarkerHigherBytes => 65432,
            SourceLens => 65421,
            XOffsetFromSlideCenter => 65422,
            YOffsetFromSlideCenter => 65423,
        }
    }
}
