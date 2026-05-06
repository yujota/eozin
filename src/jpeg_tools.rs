use self::Marker::*;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Marker {
    SOI,
    EOI,
    SOS,
    SOF0,
    SOF1,
    APP0,
    APP1,
    RST(u8), // Restart marker
    DRI,     // Define Restart Interval
    DQT,
    DHT,
    DNL,
    COM,
    Unknown(u8),
}

impl Marker {
    pub(crate) fn from_u8(n: u8) -> Marker {
        use self::Marker::*;
        match n {
            0xD0 => RST(0),
            0xD1 => RST(1),
            0xD2 => RST(2),
            0xD3 => RST(3),
            0xD4 => RST(4),
            0xD5 => RST(5),
            0xD6 => RST(6),
            0xD7 => RST(7),
            0xD8 => SOI,
            0xD9 => EOI,
            0xDA => SOS,
            0xE0 => APP0,
            0xC0 => SOF0,
            0xFE => COM,
            0xDB => DQT,
            0xDC => DNL,
            0xDD => DRI,
            0xC4 => DHT,
            _ => Unknown(n),
        }
    }
}

pub(crate) struct JpegParser<'a> {
    cur: usize,
    buf: &'a [u8],
}

#[derive(Debug, PartialEq)]
pub(crate) struct ParsedSegment {
    pub seg: Segment,
    pub offset: usize,
    pub count: usize,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Segment {
    StartOfImage,
    EndOfImage,
    App0,
    Comment(String),
    RestartInterval(usize),
    StartOfFrame0 {
        width: u16,
        height: u16,
        dt_precision: u8,
    },
    StartOfScan,
    Unknown(Marker),
}

impl<'a> JpegParser<'a> {
    pub(crate) fn parse(buf: &'a [u8]) -> Vec<ParsedSegment> {
        let mut p = Self { cur: 0, buf };
        let mut segs = Vec::new();

        loop {
            match p.parse_marker() {
                Some(m) => match p.consume_segment(m) {
                    Some(sg) => {
                        // println!("Parsing {:?}", sg);
                        segs.push(sg);
                    }
                    None => continue,
                },
                None => return segs,
            };
        }
    }

    fn consume_segment(&mut self, marker: Marker) -> Option<ParsedSegment> {
        if marker == SOI {
            let (offset, count) = (self.cur, 2);
            self.cur += 2;
            Some(ParsedSegment {
                seg: Segment::StartOfImage,
                offset,
                count,
            })
        } else if marker == EOI {
            let (offset, count) = (self.cur, 2);
            self.cur += 2;
            Some(ParsedSegment {
                seg: Segment::EndOfImage,
                offset,
                count,
            })
        } else {
            let offset = self.cur;
            self.cur += 2;
            let length = self.consume_u16()? as usize;
            let count = length + 2;
            let l = length - 2;
            let seg = match marker {
                APP0 => self.parse_app0(l),
                COM => self.parse_com(l),
                DRI => self.parse_dri(l),
                SOF0 => self.parse_sof0(l),
                SOS => self.parse_sos(l),
                _ => Some(Segment::Unknown(marker)),
            };
            self.cur += l; // Whether parse would fail or not, advance cursor.
            seg.map(|s| ParsedSegment {
                seg: s,
                count,
                offset,
            })
        }
    }

    fn parse_marker(&mut self) -> Option<Marker> {
        if !(self.cur < self.buf.len() - 1) {
            return None;
        }
        if self.buf[self.cur] == 0xFF {
            Some(Marker::from_u8(self.buf[self.cur + 1]))
        } else {
            None
        }
    }

    fn parse_app0(&mut self, _l: usize) -> Option<Segment> {
        // TODO
        Some(Segment::App0)
    }

    fn parse_com(&mut self, l: usize) -> Option<Segment> {
        self.parse_ascii(l).map(Segment::Comment)
    }

    fn parse_dri(&mut self, _l: usize) -> Option<Segment> {
        self.parse_u16()
            .map(|i| Segment::RestartInterval(i as usize))
    }

    fn parse_sof0(&mut self, l: usize) -> Option<Segment> {
        let b = &self.buf[self.cur..self.cur + l];
        let dt_precision = b[0];
        let height = u16::from_be_bytes([b[1], b[2]]);
        let width = u16::from_be_bytes([b[3], b[4]]);
        Some(Segment::StartOfFrame0 {
            width,
            height,
            dt_precision,
        })
    }

    fn parse_sos(&mut self, _l: usize) -> Option<Segment> {
        Some(Segment::StartOfScan)
    }

    fn parse_u16(&self) -> Option<u16> {
        if self.cur < self.buf.len() - 1 {
            Some(u16::from_be_bytes([
                self.buf[self.cur],
                self.buf[self.cur + 1],
            ]))
        } else {
            None
        }
    }

    fn parse_ascii(&self, n: usize) -> Option<String> {
        if self.buf.len() < n + self.cur {
            None
        } else {
            Some(
                self.buf[self.cur..self.cur + n]
                    .iter()
                    .fold(String::new(), |mut acc, x| {
                        if let Some(c) = std::char::from_u32(*x as u32) {
                            acc.push(c);
                        }
                        acc
                    }),
            )
        }
    }

    fn consume_u16(&mut self) -> Option<u16> {
        let r = self.parse_u16();
        self.cur += 2;
        r
    }
}
