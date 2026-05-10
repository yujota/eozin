use super::camera::{Camera, Lv0Pos};
use super::pyramid::{Pyramid, TileKey};
use std::collections::{HashMap, HashSet};
// use web_sys::{ImageBitmap, console};

pub type TimeDelta = f64;
pub type TimeStmap = f64;

#[derive(PartialEq, Debug, Clone)]
pub(crate) struct TileDisplay<'a, I> {
    pub(crate) img: &'a I,
    pub(crate) dx: f64,
    pub(crate) dy: f64,
    pub(crate) dw: f64,
    pub(crate) dh: f64,
}

pub(crate) struct DisplayPos {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) w: f64,
    pub(crate) h: f64,
}

#[allow(dead_code)]
pub(crate) trait Container<I> {
    fn load(&mut self, key: TileKey, img: I);
    fn tick(&mut self, perf: f64);
    fn request(&mut self) -> Option<TileKey>;
    fn update(&mut self, camera: &Camera, pyramid: &Pyramid);
    fn display(&self) -> Vec<TileDisplay<'_, I>>;
    fn is_updated(&self) -> bool;
    fn refreshed(&mut self);
}

#[allow(dead_code)]
pub(crate) struct DelayedRequestor<I> {
    req_threshold: TimeDelta,
    sweep_interval: TimeDelta,
    requesting: HashSet<TileKey>,
    to_request: Vec<TileKey>,
    images: HashMap<TileKey, I>,
    is_updated: bool,
    to_display: Vec<(TileKey, DisplayPos)>,
    missing_tiles: Vec<TileKey>,
    stay_still: Option<TimeStmap>,
    timestamp: TimeStmap,
}

impl<I> Container<I> for DelayedRequestor<I> {
    fn load(&mut self, key: TileKey, img: I) {
        self.requesting.remove(&key);
        self.images.insert(key, img);
        // self.is_updated = true;
    }
    fn request(&mut self) -> Option<TileKey> {
        /*
        if !self.to_request.is_empty() {
            console::log_1(&format!("Requesting {:?}", self.to_request.len()).into());
        }
        */
        if let Some(k) = self.to_request.pop() {
            let _ = self.requesting.insert(k);
            Some(k)
        } else {
            None
        }
    }
    fn tick(&mut self, perf: f64) {
        /*
        console::log_1(
            &format!(
                "Missing Tiles {:?}, Stay Still {:?}",
                self.missing_tiles.len(),
                self.stay_still
            )
            .into(),
        );
        */
        match self.stay_still {
            Some(ts) => {
                if (perf - ts > self.req_threshold) && !self.missing_tiles.is_empty() {
                    self.to_request = vec![];
                    std::mem::swap(&mut self.to_request, &mut self.missing_tiles);
                }
            }
            None => {
                self.stay_still = Some(perf);
            }
        }
    }
    fn display(&self) -> Vec<TileDisplay<'_, I>> {
        let mut vs: Vec<TileDisplay<'_, I>> = self
            .to_display
            .iter()
            .map(|(k, p)| TileDisplay {
                img: self.images.get(k).unwrap(),
                dx: p.x,
                dy: p.y,
                dw: p.w,
                dh: p.h,
            })
            .collect();
        vs.reverse();
        vs
    }
    fn update(&mut self, camera: &Camera, pyramid: &Pyramid) {
        // if !camera.is_updated() && !self.is_updated {
        /*
        if !camera.is_updated() {
            return;
        };
        */
        // console::log_1(&format!("Is camera updated {:?}", camera.is_updated()).into());
        self.is_updated = true;
        self.stay_still = None;
        let mut targets: Vec<(Lv0Pos, Lv0Pos)> = vec![(camera.top_left(), camera.bottom_right())];
        let mut to_display = vec![];

        let best_lv = pyramid.fined_level(camera.zoom_factor());
        // console::log_1(&format!("Best Lv {:?}",  best_lv).into());
        let mut current_lv_missing = vec![];
        let zf_cam = camera.zoom_factor();
        let top_left_cam = camera.top_left();
        let (cx, cy) = (top_left_cam.0 / zf_cam, top_left_cam.1 / zf_cam);
        // console::log_1(&format!("M0 {:?}", camera.is_updated()).into());
        for l in best_lv..pyramid.num_level() {
            // console::log_1(&format!("M1 {:?}", camera.is_updated()).into());
            let zf_lv = pyramid.zoom_factor(l);
            let r_zf = zf_lv / zf_cam;
            let (tw, th) = pyramid.tile_size(l);
            // let (mtw, mth) = pyramid.marginal_tile_size(l);
            let mut next_target = Vec::new();
            // console::log_1(&format!("M2 {:?}", camera.is_updated()).into());
            for t in targets {
                // console::log_1(&format!("M3 {:?}", &t).into());
                let r = pyramid.target_range(t.0, t.1, l);
                // console::log_1(&format!("M4 {:?}", &r).into());
                for key in r.to_keys() {
                    // console::log_1(&format!("M5 {:?}", &key).into());
                    let (c_tw, c_th) = pyramid.get_tile_size(l, key.x, key.y);
                    if self.images.contains_key(&key) {
                        let pos = DisplayPos {
                            x: -cx + tw * key.x as f64 * r_zf,
                            y: -cy + th * key.y as f64 * r_zf,
                            w: c_tw * r_zf,
                            h: c_th * r_zf,
                        };
                        to_display.push((key, pos));
                    } else {
                        let tp_lft = (tw * key.x as f64 * zf_lv, th * key.y as f64 * zf_lv);
                        let btm_rgt = (
                            tw * (key.x + 1) as f64 * zf_lv,
                            th * (key.y + 1) as f64 * zf_lv,
                        );
                        next_target.push((tp_lft, btm_rgt));
                        if l == best_lv && !self.requesting.contains(&key) {
                            current_lv_missing.push(key);
                        }
                    }
                }
            }
            // console::log_1(&format!("M3 {:?}", camera.is_updated()).into());
            if l == best_lv {
                let lv0_center = camera.center();
                current_lv_missing
                    .sort_by_key(|k| k.sq_distance(tw, th, zf_lv, lv0_center).floor() as usize);
                current_lv_missing.reverse();
            }
            targets = next_target;
        }
        // console::log_1(&format!("M3 {:?}", camera.is_updated()).into());
        self.missing_tiles = current_lv_missing;
        // console::log_1(&format!("Missing Tiles {:?}", self.missing_tiles.len()).into());
        self.to_display = to_display;
        // self.is_updated = false;
    }
    fn is_updated(&self) -> bool {
        self.is_updated
    }
    fn refreshed(&mut self) {
        self.is_updated = false;
    }
}

pub(crate) struct DelayedRequestorBuilder {}

impl DelayedRequestorBuilder {
    pub(crate) fn new() -> DelayedRequestorBuilder {
        DelayedRequestorBuilder {}
    }
    pub(crate) fn build<I>(self) -> DelayedRequestor<I> {
        DelayedRequestor {
            req_threshold: 30.0,
            sweep_interval: 300.0,
            requesting: HashSet::new(),
            to_request: vec![],
            images: HashMap::new(),
            is_updated: false,
            to_display: vec![],
            missing_tiles: vec![],
            stay_still: None,
            timestamp: -10000000000.0,
        }
    }
}
