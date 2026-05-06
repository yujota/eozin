#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, Copy, Clone, Eq)]
pub enum CommonTag {
    NewSubfileType,
    SubfileType,
    ImageWidth,
    ImageLength,
    BitsPerSample,
    Compression,
    PhotometricInterpretation,
    Threshholding,
    CellWidth,
    CellLength,
    FillOrder,
    DocumentName,
    ImageDescription,
    Make,
    Model,
    StripOffsets,
    Orientation,
    SamplesPerPixel,
    RowsPerStrip,
    StripByteCounts,
    MinSampleValue,
    MaxSampleValue,
    XResolution,
    YResolution,
    PlanarConfiguration,
    PageName,
    XPosition,
    YPosition,
    FreeOffset,
    FreeByteCounts,
    GrayResponseUnit,
    GrayResponseCurve,
    ResolutionUnit,
    PageNumber,
    Software,
    DateTime,
    Artist,
    HostComputer,
    Predictor,
    ColorMap,
    TileWidth,
    TileLength,
    TileOffsets,
    TileByteCounts,
    JPEGTables,

    JPEGProc,
    JPEGInterchangeFormat,
    JPEGInterchangeFormatLength,
    JPEGRestartInterval,
    JPEGLosslessPredictors,
    JPEGPointTransform,
    JPEGQTables,
    JPEGDCTables,
    JPEGACTables,

    YCbCrCoefficients,
    YCbCrSubSampling,
    YCbCrPositioning,

    ReferenceBlackWhite,

    XMP,
    Copyright,

    UndefinedTag(u16),
}

impl From<CommonTag> for u16 {
    fn from(tag: CommonTag) -> u16 {
        use CommonTag::*;
        match tag {
            NewSubfileType => 254,
            SubfileType => 255,
            ImageWidth => 256,
            ImageLength => 257,
            BitsPerSample => 258,
            Compression => 259,
            PhotometricInterpretation => 262,
            Threshholding => 263,
            CellWidth => 264,
            CellLength => 265,
            FillOrder => 266,
            DocumentName => 269,
            ImageDescription => 270,
            Make => 271,
            Model => 272,
            StripOffsets => 273,
            Orientation => 274,
            SamplesPerPixel => 277,
            RowsPerStrip => 278,
            StripByteCounts => 279,
            MinSampleValue => 280,
            MaxSampleValue => 281,
            XResolution => 282,
            YResolution => 283,
            PlanarConfiguration => 284,
            PageName => 285,
            XPosition => 286,
            YPosition => 287,
            FreeOffset => 288,
            FreeByteCounts => 289,
            GrayResponseUnit => 290,
            GrayResponseCurve => 291,
            ResolutionUnit => 296,
            PageNumber => 297,
            Software => 305,
            DateTime => 306,
            Artist => 315,
            HostComputer => 316,
            Predictor => 317,
            ColorMap => 320,
            TileWidth => 322,
            TileLength => 323,
            TileOffsets => 324,
            TileByteCounts => 325,
            JPEGTables => 347,
            JPEGProc => 512,
            JPEGInterchangeFormat => 513,
            JPEGInterchangeFormatLength => 514,
            JPEGRestartInterval => 515,
            JPEGLosslessPredictors => 517,
            JPEGPointTransform => 518,
            JPEGQTables => 519,
            JPEGDCTables => 520,
            JPEGACTables => 521,
            YCbCrCoefficients => 529,
            YCbCrSubSampling => 530,
            YCbCrPositioning => 531,
            ReferenceBlackWhite => 532,
            XMP => 700,
            Copyright => 33432,

            UndefinedTag(n) => n,
        }
    }
}

impl From<u16> for CommonTag {
    fn from(n: u16) -> CommonTag {
        use CommonTag::*;
        match n {
            254 => NewSubfileType,
            255 => SubfileType,
            256 => ImageWidth,
            257 => ImageLength,
            258 => BitsPerSample,
            259 => Compression,
            262 => PhotometricInterpretation,
            263 => Threshholding,
            264 => CellWidth,
            265 => CellLength,
            266 => FillOrder,
            269 => DocumentName,
            270 => ImageDescription,
            271 => Make,
            272 => Model,
            273 => StripOffsets,
            274 => Orientation,
            277 => SamplesPerPixel,
            278 => RowsPerStrip,
            279 => StripByteCounts,
            280 => MinSampleValue,
            281 => MaxSampleValue,
            282 => XResolution,
            283 => YResolution,
            284 => PlanarConfiguration,
            285 => PageName,
            286 => XPosition,
            287 => YPosition,
            288 => FreeOffset,
            289 => FreeByteCounts,
            290 => GrayResponseUnit,
            291 => GrayResponseCurve,
            296 => ResolutionUnit,
            297 => PageNumber,
            305 => Software,
            306 => DateTime,
            315 => Artist,
            316 => HostComputer,
            317 => Predictor,
            320 => ColorMap,
            322 => TileWidth,
            323 => TileLength,
            324 => TileOffsets,
            325 => TileByteCounts,

            347 => JPEGTables,

            512 => JPEGProc,
            513 => JPEGInterchangeFormat,
            514 => JPEGInterchangeFormatLength,
            515 => JPEGRestartInterval,
            517 => JPEGLosslessPredictors,
            518 => JPEGPointTransform,
            519 => JPEGQTables,
            520 => JPEGDCTables,
            521 => JPEGACTables,

            529 => YCbCrCoefficients,
            530 => YCbCrSubSampling,
            531 => YCbCrPositioning,

            532 => ReferenceBlackWhite,

            700 => XMP,
            33432 => Copyright,

            _ => UndefinedTag(n),
        }
    }
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
    MetadataBlock, // 65423 	YOffsetFromSlideCentre
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
            MetadataBlock => 65449,
        }
    }
}
