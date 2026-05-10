use crate::camera::CameraBuilder;
use crate::pyramid::PyramidBuilder;

use super::camera::Camera;
use super::container::{Container, DelayedRequestor, DelayedRequestorBuilder};
use super::pyramid::{LevelInfo, Pyramid, TileKey};
use eozin::wasm::web::DynamicDecoder;
use wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, ImageBitmap};

pub(crate) fn tick(perf: f64) -> Msg {
    Msg::OnAnimationFrame(perf)
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Msg {
    OnMouseMove(MouseMove),
    OnMouseDown(MouseDown),
    OnMouseUp,
    OnWheel(MouseWheel),
    OnTileRead((TileKey, ImageBitmap)),
    OnAnimationFrame(f64),
    OnSingleFingerStart(SingleFingerStart),
    OnTwoFingerStart(TwoFingerStart),
    OnSingleFingerMove(SingleFingerMove),
    OnTwoFingerMove(TwoFingerMove),
    OnSingleFingerEnd,
    OnTwoFingerEnd,
}

#[derive(Debug)]
pub(crate) enum Cmd {
    NoCmd,
    ReadTile(TileKey),
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub(crate) struct MouseDown {
    pub offset_x: i32,
    pub offset_y: i32,
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub(crate) struct MouseMove {
    pub offset_x: i32,
    pub offset_y: i32,
}

#[derive(PartialEq, Debug, Clone)]
pub(crate) struct MouseWheel {
    pub offset_x: i32,
    pub offset_y: i32,
    pub delta_y: f64,
}

// Two pointer Event
#[derive(PartialEq, Debug, Clone)]
pub(crate) struct SingleFingerStart {
    pub offset_x: f64,
    pub offset_y: f64,
}

// Two pointer Event
#[derive(PartialEq, Debug, Clone)]
pub(crate) struct TwoFingerStart {
    pub offset_x: f64,
    pub offset_y: f64,
    // Square Distance of two pointers
    pub distance: f64,
}

// Two pointer Event
#[derive(PartialEq, Debug, Clone)]
pub(crate) struct SingleFingerMove {
    pub offset_x: f64,
    pub offset_y: f64,
}

// Two pointer Event
#[derive(PartialEq, Debug, Clone)]
pub(crate) struct TwoFingerMove {
    pub offset_x: f64,
    pub offset_y: f64,
    // Square Distance of two pointers
    pub distance: f64,
}

pub(crate) struct Model {
    mouse_pressed: bool,
    refresh: bool,
    camera: Camera,
    touch: TouchState,
    tiles: DelayedRequestor<ImageBitmap>,
    pyramid: Pyramid,
}

enum TouchState {
    Idling,
    Panning,
    Zooming {
        distance: f64,
        x: f64,
        y: f64,
        original_zoom_exp: f64,
    },
}

pub(crate) struct ModelBuilder {
    canvas_size: (f64, f64),
    tile_display_tol: f64,
}

impl ModelBuilder {
    pub(crate) fn new() -> ModelBuilder {
        ModelBuilder {
            canvas_size: (1200.0, 900.0),
            tile_display_tol: 0.75,
        }
    }
    pub(crate) fn set_canvas_size(self, canvas_size: (f64, f64)) -> ModelBuilder {
        ModelBuilder {
            canvas_size,
            ..self
        }
    }

    pub(crate) fn build(self, decoder: &DynamicDecoder) -> Model {
        let dims = decoder.level_dimensions();
        let lv0 = dims.first().unwrap();
        let mut camera =
            CameraBuilder::new().build(self.canvas_size, (lv0.width as f64, lv0.height as f64));
        camera.fit_to_frame();
        let mut levels = vec![];
        let t_sizes = decoder.level_tile_sizes();
        // let ms = decoder.level_marginal_tile_sizes();
        for (d, t) in dims.iter().zip(t_sizes.iter()) {
            let mw = d.width % t.width;
            let mh = d.height % t.height;
            let marginal_tile_width = if mw == 0 { None } else { Some(mw as u64) };
            let marginal_tile_height = if mh == 0 { None } else { Some(mh as u64) };
            let lv = LevelInfo {
                width: d.width as u64,
                height: d.height as u64,
                tile_width: t.width as u64,
                tile_height: t.height as u64,
                magnification: None,
                marginal_tile_width,
                marginal_tile_height,
            };
            levels.push(lv);
        }
        let pyramid = PyramidBuilder::new()
            .set_tile_display_blur_tolerance(self.tile_display_tol)
            .build(&levels);
        // let pyramid = Pyramid::new(&levels, self.tile_display_tol);
        let mut tiles = DelayedRequestorBuilder::new().build();
        tiles.update(&camera, &pyramid);
        Model {
            mouse_pressed: false,
            refresh: false,
            touch: TouchState::new(),
            camera,
            tiles,
            pyramid,
        }
    }
}

#[allow(dead_code)]
impl Model {
    pub(crate) fn is_mouse_pressed(&self) -> bool {
        self.mouse_pressed
    }
    pub(crate) fn tick(&mut self, perf: f64) {
        // console::log_1(&"Ticking".into());
        self.tiles.tick(perf);
    }
    pub(crate) fn update(&mut self, msg: Msg) -> Cmd {
        // console::log_1(&format!("Is camera updated {:?}", self.camera.is_updated()).into());

        use Msg::*;
        match msg {
            OnMouseUp => {
                self.mouse_pressed = false;
            }
            OnMouseDown(MouseDown { offset_x, offset_y }) => {
                self.mouse_pressed = true;
                self.camera.move_start(offset_x as f64, offset_y as f64);
            }
            OnMouseMove(MouseMove { offset_x, offset_y }) => {
                self.camera.update_move(offset_x as f64, offset_y as f64);
                self.tiles.update(&self.camera, &self.pyramid);
                self.camera.applied();
                self.refresh = true;
            }
            OnWheel(MouseWheel {
                offset_x,
                offset_y,
                delta_y,
            }) => {
                if delta_y < 0.0 {
                    self.camera.zoom_in(offset_x as f64, offset_y as f64)
                } else {
                    self.camera.zoom_out(offset_x as f64, offset_y as f64)
                };
                self.tiles.update(&self.camera, &self.pyramid);
                self.camera.applied();
                self.refresh = true;
            }
            OnTileRead((key, img)) => {
                // console::log_1(&format!("TIle load  {:?}", &key).into());
                self.tiles.load(key, img);
                self.tiles.update(&self.camera, &self.pyramid);
                self.refresh = true;
            }
            OnAnimationFrame(perf) => {
                self.tiles.tick(perf);
            }
            OnSingleFingerStart(ev) => {
                self.touch.single_start(&mut self.camera, ev);
                self.tiles.update(&self.camera, &self.pyramid);
                self.camera.applied();
                self.refresh = true;
            }
            OnTwoFingerStart(ev) => {
                self.touch.double_start(&mut self.camera, ev);
                self.tiles.update(&self.camera, &self.pyramid);
                self.camera.applied();
                self.refresh = true;
            }
            OnSingleFingerMove(ev) => {
                self.touch.single_move(&mut self.camera, ev);
                self.tiles.update(&self.camera, &self.pyramid);
                self.camera.applied();
                self.refresh = true;
            }
            OnTwoFingerMove(ev) => {
                self.touch.double_move(&mut self.camera, ev);
                self.tiles.update(&self.camera, &self.pyramid);
                self.camera.applied();
                self.refresh = true;
            }
            OnSingleFingerEnd => {
                self.touch.single_end();
            }
            OnTwoFingerEnd => {
                self.touch.double_end();
            }
        };
        self.tiles
            .request()
            .map(Cmd::ReadTile)
            .unwrap_or(Cmd::NoCmd)
    }
    pub(crate) fn view(&mut self, canvas: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        // console::log_1(&format!("refresh view {}", self.refresh_view).into());
        // console::log_1(&format!("Is updated {:?}", self.tiles.is_updated()).into());
        if self.refresh {
            let (cw, ch) = self.camera.canvas_size();
            canvas.reset();
            canvas.clear_rect(0.0, 0.0, cw, ch);
            canvas.fill();
            for d in self.tiles.display() {
                canvas.draw_image_with_image_bitmap_and_dw_and_dh(d.img, d.dx, d.dy, d.dw, d.dh)?;
            }
            // canvas.fill();
        }
        self.refresh = false;
        Ok(())
    }
}

impl TouchState {
    fn new() -> TouchState {
        Self::Idling
    }
    fn single_start(&mut self, camera: &mut Camera, ev: SingleFingerStart) {
        camera.move_start(ev.offset_x, ev.offset_y);
        *self = Self::Panning;
    }
    fn double_start(&mut self, camera: &mut Camera, ev: TwoFingerStart) {
        *self = Self::Zooming {
            distance: ev.distance,
            x: ev.offset_x,
            y: ev.offset_y,
            original_zoom_exp: camera.zoom_exp(),
        };
    }
    fn single_move(&mut self, camera: &mut Camera, ev: SingleFingerMove) {
        if matches!(self, Self::Panning) {
            camera.update_move(ev.offset_x, ev.offset_y);
        }
    }
    fn double_move(&mut self, camera: &mut Camera, ev: TwoFingerMove) {
        if let Self::Zooming {
            distance,
            x,
            y,
            original_zoom_exp,
        } = self
        {
            let zoom_ratio = ev.distance / *distance;
            let zoom_exp_delta = zoom_ratio.log2();
            let new_zoom_exp = *original_zoom_exp - zoom_exp_delta;
            camera.zoom_with(*x, *y, new_zoom_exp);
        }
    }
    fn single_end(&mut self) {
        *self = Self::Idling;
    }
    fn double_end(&mut self) {
        *self = Self::Idling;
    }
}
