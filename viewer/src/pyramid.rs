use crate::camera::{Lv0Pos, ZoomFactor};

#[derive(Debug, Clone)]
pub(crate) struct PyramidBuilder {
    tile_display_blur_tolerance: f64,
    magnification_round_gap: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct Pyramid {
    levels: Vec<Level>,
    zf_tol: f64,
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Copy)]
pub(crate) struct TileKey {
    pub x: usize,
    pub y: usize,
    pub lv: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct Level {
    pub width: u64,
    pub height: u64,
    pub tile_width: u64,
    pub tile_height: u64,
    pub tile_range_x: usize,
    pub tile_range_y: usize,
    pub marginal_tile_width: Option<u64>,
    pub marginal_tile_height: Option<u64>,
    pub zoom_factor: ZoomFactor,
}

pub(crate) struct LevelInfo {
    pub width: u64,
    pub height: u64,
    pub tile_width: u64,
    pub tile_height: u64,
    pub magnification: Option<f64>,
    pub marginal_tile_width: Option<u64>,
    pub marginal_tile_height: Option<u64>,
}

/// Helper structs and implementations
#[derive(Debug, Clone)]
pub(crate) struct TileRange {
    pub x0: usize,
    pub y0: usize,
    pub x1: usize,
    pub y1: usize,
    pub lv: usize,
}

impl PyramidBuilder {
    pub(crate) fn new() -> PyramidBuilder {
        PyramidBuilder {
            tile_display_blur_tolerance: 0.75,
            magnification_round_gap: 0.05,
        }
    }
    pub(crate) fn set_tile_display_blur_tolerance(self, t: f64) -> PyramidBuilder {
        PyramidBuilder {
            tile_display_blur_tolerance: t,
            ..self
        }
    }
    pub(crate) fn build(self, levels: &[LevelInfo]) -> Pyramid {
        let lv0 = levels.first().unwrap();
        let (lv0_w, lv0_h) = (lv0.width, lv0.height);
        let mut lvs = vec![];
        let mut r_count = 0;
        let mut r_base: f64 = 0.0;
        for (i, lv) in levels.iter().enumerate() {
            let tile_range_x = da(lv.width, lv.tile_width) as usize;
            let tile_range_y = da(lv.height, lv.tile_height) as usize;
            let zoom_factor = if let Some(zf) = lv.magnification {
                zf
            } else if i == 0 {
                1.0
            } else if r_count > 4 {
                r_base.powf(i as f64)
            } else {
                let r =
                    ((lv0_w as f64 / lv.width as f64) + (lv0_h as f64 / lv.height as f64)) / 2.0;
                let base = r.powf(1.0 / i as f64).round();
                if (base + self.magnification_round_gap).powf(i as f64) > r
                    && r > (base - self.magnification_round_gap).powf(i as f64)
                {
                    if r_base == base {
                        r_count += 1;
                    } else if r_count == 0 {
                        r_base = base;
                        r_count += 1;
                    } else {
                        r_count = -10000;
                    };
                    base.powf(i as f64)
                } else {
                    r
                }
            };
            // console::log_1(&format!("ZoomFactor {:?}", zoom_factor).into());
            let l = Level {
                width: lv.width,
                height: lv.height,
                tile_width: lv.tile_width,
                tile_height: lv.tile_height,
                marginal_tile_width: lv.marginal_tile_width,
                marginal_tile_height: lv.marginal_tile_height,
                tile_range_x,
                tile_range_y,
                zoom_factor,
            };
            lvs.push(l)
        }
        Pyramid {
            zf_tol: self.tile_display_blur_tolerance,
            levels: lvs,
        }
    }
}

impl Pyramid {
    pub(crate) fn zoom_factor(&self, lv: usize) -> ZoomFactor {
        self.levels.get(lv).unwrap().zoom_factor
    }
    pub(crate) fn tile_size(&self, lv: usize) -> (f64, f64) {
        let lv = self.levels.get(lv).unwrap();
        (lv.tile_width as f64, lv.tile_height as f64)
    }
    pub(crate) fn marginal_tile_size(&self, lv: usize) -> (Option<f64>, Option<f64>) {
        let lv = self.levels.get(lv).unwrap();
        (
            lv.marginal_tile_width.map(|i| i as f64),
            lv.marginal_tile_height.map(|i| i as f64),
        )
    }
    pub(crate) fn get_tile_size(&self, lv: usize, x: usize, y: usize) -> (f64, f64) {
        let l = self.levels.get(lv).unwrap();
        let (tw, th) = (l.tile_width as f64, l.tile_height as f64);
        let (mw, mh) = self.marginal_tile_size(lv);
        let mw = mw.unwrap_or(tw);
        let mh = mh.unwrap_or(th);
        if x < l.tile_range_x - 1 && y < l.tile_range_y - 1 {
            (tw, th)
        } else if x == l.tile_range_x - 1 && y < l.tile_range_y - 1 {
            (mw, th)
        } else if x < l.tile_range_x - 1 && y == l.tile_range_y - 1 {
            (tw, mh)
        } else {
            (mw, mh)
        }
    }
    pub(crate) fn target_range(
        &self,
        top_left: Lv0Pos,
        bottom_right: Lv0Pos,
        lv: usize,
    ) -> TileRange {
        let l = lv;
        let lv = self.levels.get(lv).unwrap();
        let tw = lv.tile_width as f64 * lv.zoom_factor;
        let th = lv.tile_height as f64 * lv.zoom_factor;
        let x0 = (top_left.0 / tw).floor() as usize;
        let y0 = (top_left.1 / th).floor() as usize;
        let x1 = (bottom_right.0 / tw).floor() as usize;
        let y1 = (bottom_right.1 / th).floor() as usize;
        let x1 = (x1 + 2).min(lv.tile_range_x);
        let y1 = (y1 + 2).min(lv.tile_range_y);
        TileRange {
            x0,
            y0,
            x1,
            y1,
            lv: l,
        }
    }

    pub(crate) fn fined_level(&self, zoom_factor: ZoomFactor) -> usize {
        self.levels
            .iter()
            .enumerate()
            .rev()
            .filter_map(|(i, lv)| {
                if lv.zoom_factor < zoom_factor - self.zf_tol {
                    Some(i)
                } else {
                    None
                }
            })
            .next()
            .unwrap_or(0)
    }
    pub(crate) fn num_level(&self) -> usize {
        self.levels.len()
    }
}

impl TileKey {
    pub(crate) fn sq_distance(
        &self,
        tw: f64,
        th: f64,
        zoom_factor_lv: f64,
        lv0_center: Lv0Pos,
    ) -> f64 {
        let x = self.x as f64 * tw * zoom_factor_lv;
        let y = self.y as f64 * th * zoom_factor_lv;
        (x - lv0_center.0).powf(2.0) + (y - lv0_center.1).powf(2.0)
    }
}

impl TileRange {
    pub(crate) fn to_keys(&self) -> Vec<TileKey> {
        let mut vs = vec![];
        for y in self.y0..self.y1 {
            for x in self.x0..self.x1 {
                let key = TileKey { lv: self.lv, x, y };
                vs.push(key);
            }
        }
        vs
    }
}
#[allow(clippy::manual_is_multiple_of)]
fn da(a: u64, b: u64) -> u64 {
    if a % b == 0 { a / b } else { a / b + 1 }
}
