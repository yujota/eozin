use super::pyramid::TileKey;
use super::wasm_model::{
    Cmd, Model, ModelBuilder, MouseDown, MouseMove, MouseWheel,
    Msg::{self, *},
    SingleFingerMove, SingleFingerStart, TwoFingerMove, TwoFingerStart, tick,
};
use eozin::wasm::web::DynamicDecoder;
use futures;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsError, JsValue, closure::Closure, prelude::wasm_bindgen};
use web_sys::{
    AddEventListenerOptions, Blob, CanvasRenderingContext2d, File, HtmlCanvasElement, MouseEvent,
    TouchEvent, WheelEvent, Worker, console,
};

type LoopClosure = Closure<dyn FnMut(f64)>;
type Sender = futures::channel::mpsc::UnboundedSender<Msg>;
type Receiver = futures::channel::mpsc::UnboundedReceiver<Msg>;

#[wasm_bindgen]
pub struct EozinViewer {
    sender: Sender,
    receiver: Option<Receiver>,
    model: Rc<RefCell<Model>>,
    workers: Vec<Worker>,
    is_started: bool,
    canvas_id: String,
}

#[wasm_bindgen]
pub struct EozinViewerBuilder {
    canvas_id: String,
    canvas_size: Option<(f64, f64)>,
    workers: Vec<Worker>,
}

#[wasm_bindgen]
impl EozinViewerBuilder {
    #[allow(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        EozinViewerBuilder {
            canvas_id: "eozin-viewer-canvas".to_string(),
            workers: vec![],
            canvas_size: None,
        }
    }
    #[wasm_bindgen]
    pub fn set_canvas_id(mut self, canvas_id: &str) -> Self {
        self.canvas_id = canvas_id.to_string();
        self
    }
    #[wasm_bindgen]
    pub fn set_worker(mut self, worker: &Worker) -> Self {
        self.workers.push(worker.clone());
        self
    }
    #[wasm_bindgen(js_name = setCanvasSize)]
    pub fn set_canvas_size(mut self, canvas_width: f64, canvas_height: f64) -> Self {
        self.canvas_size = Some((canvas_width, canvas_height));
        self
    }

    #[wasm_bindgen]
    pub async fn build_with_file(self, file: File) -> Result<EozinViewer, JsError> {
        // console::log_1(&"Starting building..".to_string().into());
        let (sender, receiver) = futures::channel::mpsc::unbounded::<Msg>();
        for w in self.workers.iter() {
            // console::log_1(&"Setting worker".to_string().into());
            Self::set_worker_event(w, sender.clone())?;
            let msg = WorkerMsg::InitWithFile(file.clone());
            // console::log_1(&format!("Dispatching worker msg {:?}", &msg).into());
            let _ = w.post_message(&msg.to_js_value());
        }
        let canvas = Self::get_canvas(&self.canvas_id)?;
        let EozinViewerBuilder {
            workers,
            canvas_id,
            canvas_size,
        } = self;
        let decoder = DynamicDecoder::with_file(file).await.map_err(|s| {
            console::log_1(&format!("Err {:?}", s).into());
            JsError::new(&s.to_string())
        })?;

        let canvas_size = canvas_size.unwrap_or((canvas.width() as f64, canvas.height() as f64));
        let model = ModelBuilder::new()
            .set_canvas_size((canvas_size.0, canvas_size.1))
            .build(&decoder);
        let model = Rc::new(RefCell::new(model));
        Self::set_mouse_event(&canvas, model.clone(), sender.clone())?;
        Self::set_touch_event(&canvas, sender.clone())?;
        Ok(EozinViewer {
            workers,
            canvas_id,
            sender,
            receiver: Some(receiver),
            model,
            is_started: false,
        })
    }

    fn get_canvas(canvas_id: &str) -> Result<HtmlCanvasElement, JsError> {
        let window = web_sys::window().unwrap();
        // let dpr = window.device_pixel_ratio();
        // console::log_1(&format!("DPR {:?}", dpr).to_string().into());
        let document = window.document().unwrap();
        let canvas = document
            .get_element_by_id(canvas_id)
            .ok_or(JsError::new("Selected canvas id is not found"))?;
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| JsError::new("Failed to get HTML Canvas"))?;
        Ok(canvas)
    }

    fn set_mouse_event(
        canvas: &HtmlCanvasElement,
        model: Rc<RefCell<Model>>,
        sender: Sender,
    ) -> Result<(), JsError> {
        let (m, s) = (model.clone(), sender.clone());
        let on_move = Closure::<dyn FnMut(_)>::new(move |event: MouseEvent| {
            if m.borrow().is_mouse_pressed() {
                let _ = s.unbounded_send(OnMouseMove(MouseMove {
                    offset_x: event.offset_x(),
                    offset_y: event.offset_y(),
                }));
            };
        });
        let s = sender.clone();
        let on_press = Closure::<dyn FnMut(_)>::new(move |event: MouseEvent| {
            let _ = s.unbounded_send(Msg::OnMouseDown(MouseDown {
                offset_x: event.offset_x(),
                offset_y: event.offset_y(),
            }));
        });
        let s = sender.clone();
        let on_release = Closure::<dyn FnMut(_)>::new(move |_event: MouseEvent| {
            let _ = s.unbounded_send(Msg::OnMouseUp);
        });
        let s = sender.clone();
        let on_leave = Closure::<dyn FnMut(_)>::new(move |_event: MouseEvent| {
            let _ = s.unbounded_send(Msg::OnMouseUp);
        });
        let s = sender.clone();
        let on_wheel = Closure::<dyn FnMut(_)>::new(move |event: WheelEvent| {
            event.prevent_default();
            let _ = s.unbounded_send(Msg::OnWheel(MouseWheel {
                offset_x: event.offset_x(),
                offset_y: event.offset_y(),
                delta_y: event.delta_y(),
            }));
        });
        canvas.set_onmousedown(Some(on_press.as_ref().unchecked_ref()));
        canvas.set_onmouseup(Some(on_release.as_ref().unchecked_ref()));
        canvas.set_onmousemove(Some(on_move.as_ref().unchecked_ref()));
        canvas.set_onmouseleave(Some(on_leave.as_ref().unchecked_ref()));
        canvas.set_onwheel(Some(on_wheel.as_ref().unchecked_ref()));
        on_move.forget();
        on_press.forget();
        on_release.forget();
        on_leave.forget();
        on_wheel.forget();
        Ok(())
    }

    #[allow(clippy::collapsible_if)]
    fn set_touch_event(canvas: &HtmlCanvasElement, sender: Sender) -> Result<(), JsError> {
        let touches_id: Rc<RefCell<HashSet<i32>>> = Rc::new(RefCell::new(HashSet::new()));
        let lst_opt = AddEventListenerOptions::new();
        lst_opt.set_passive(false);
        let (ts, s, c) = (touches_id.clone(), sender.clone(), canvas.clone());
        let on_touch_start = Closure::<dyn FnMut(_)>::new(move |event: TouchEvent| {
            let n = ts.borrow().len();
            let changed = event.changed_touches();
            for i in 0..changed.length() {
                let touch = changed.get(i).unwrap();
                let _ = ts.borrow_mut().insert(touch.identifier());
            }
            let touches = event.touches();
            let ln = touches.length();
            if ln == 2 && n <= 1 {
                event.prevent_default();
                let t0 = touches.item(0).unwrap();
                let t1 = touches.item(1).unwrap();
                let (x0, y0) = (t0.client_x() as f64, t0.client_y() as f64);
                let (x1, y1) = (t1.client_x() as f64, t1.client_y() as f64);
                let (x, y) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                let distance = ((x0 - x1).powf(2.0) + (y0 - y1).powf(2.0)).sqrt();
                let rect = c.get_bounding_client_rect();
                let _ = s.unbounded_send(Msg::OnTwoFingerStart(TwoFingerStart {
                    offset_x: x - rect.x(),
                    offset_y: y - rect.y(),
                    distance,
                }));
            } else if ln == 1 && n == 0 {
                event.prevent_default();
                let t0 = touches.item(0).unwrap();
                let (x, y) = (t0.client_x() as f64, t0.client_y() as f64);
                let rect = c.get_bounding_client_rect();
                let _ = s.unbounded_send(Msg::OnSingleFingerStart(SingleFingerStart {
                    offset_x: x - rect.x(),
                    offset_y: y - rect.y(),
                }));
            }
        });
        canvas
            .add_event_listener_with_callback_and_add_event_listener_options(
                "touchstart",
                on_touch_start.as_ref().unchecked_ref(),
                &lst_opt,
            )
            .expect("failed to add touchstart listener");
        on_touch_start.forget();

        let (ts, s, _c) = (touches_id.clone(), sender.clone(), canvas.clone());
        let on_touch_end = Closure::<dyn FnMut(_)>::new(move |event: TouchEvent| {
            let n = ts.borrow().len();
            let changed = event.changed_touches();
            for i in 0..changed.length() {
                let touch = changed.get(i).unwrap();
                let _ = ts.borrow_mut().remove(&touch.identifier());
            }
            let m = ts.borrow().len();
            if n == 2 && m <= 1 {
                event.prevent_default();
                let _ = s.unbounded_send(Msg::OnTwoFingerEnd);
            } else if n == 1 && m == 0 {
                event.prevent_default();
                let _ = s.unbounded_send(Msg::OnSingleFingerEnd);
            }
        });
        canvas
            .add_event_listener_with_callback_and_add_event_listener_options(
                "touchend",
                on_touch_end.as_ref().unchecked_ref(),
                &lst_opt,
            )
            .expect("failed to add touchstart listener");
        on_touch_end.forget();

        let (ts, s, c) = (touches_id.clone(), sender.clone(), canvas.clone());
        let on_touch_move = Closure::<dyn FnMut(_)>::new(move |event: TouchEvent| {
            event.prevent_default();
            let n = ts.borrow().len();
            if n == 1 {
                let changed = event.changed_touches();
                if let Some(t) = changed.get(0) {
                    if ts.borrow().contains(&t.identifier()) {
                        let (x, y) = (t.client_x() as f64, t.client_y() as f64);
                        let rect = c.get_bounding_client_rect();
                        let _ = s.unbounded_send(Msg::OnSingleFingerMove(SingleFingerMove {
                            offset_x: x - rect.x(),
                            offset_y: y - rect.y(),
                        }));
                    }
                }
            } else if n == 2 {
                let touches = event.touches();
                if let (Some(t0), Some(t1)) = (touches.get(0), touches.get(1)) {
                    if ts.borrow().contains(&t0.identifier())
                        && ts.borrow().contains(&t1.identifier())
                    {
                        let (x0, y0) = (t0.client_x() as f64, t0.client_y() as f64);
                        let (x1, y1) = (t1.client_x() as f64, t1.client_y() as f64);
                        let (x, y) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                        let distance = ((x0 - x1).powf(2.0) + (y0 - y1).powf(2.0)).sqrt();
                        let rect = c.get_bounding_client_rect();
                        let _ = s.unbounded_send(Msg::OnTwoFingerMove(TwoFingerMove {
                            offset_x: x - rect.x(),
                            offset_y: y - rect.y(),
                            distance,
                        }));
                    }
                }
            }
        });
        canvas
            .add_event_listener_with_callback_and_add_event_listener_options(
                "touchmove",
                on_touch_move.as_ref().unchecked_ref(),
                &lst_opt,
            )
            .expect("failed to add touchstart listener");
        on_touch_move.forget();
        Ok(())
    }

    fn set_worker_event(worker: &Worker, sender: Sender) -> Result<(), JsError> {
        let on_message = Closure::<dyn FnMut(_)>::new(move |msg: JsValue| {
            // console::log_1(&format!("Got message from worker {:?}", &msg).into());
            if msg.is_null() {
                return;
            };
            if let Ok(msg) = js_sys::Reflect::get(&msg, &"data".into()) {
                // console::log_1(&format!("Got message from worker {:?}", &msg).into());
                let lv = js_sys::Reflect::get(&msg, &"lv".into())
                    .ok()
                    .and_then(|x| x.as_f64())
                    .map(|x| x.round() as usize);
                let x = js_sys::Reflect::get(&msg, &"x".into())
                    .ok()
                    .and_then(|x| x.as_f64())
                    .map(|x| x.round() as usize);
                let y = js_sys::Reflect::get(&msg, &"y".into())
                    .ok()
                    .and_then(|x| x.as_f64())
                    .map(|x| x.round() as usize);
                let bitmap = js_sys::Reflect::get(&msg, &"img".into()).ok();
                if let (Some(lv), Some(x), Some(y), Some(bitmap)) = (lv, x, y, bitmap) {
                    let tile_key = TileKey { x, y, lv };
                    let _ = sender.unbounded_send(OnTileRead((tile_key, bitmap.unchecked_into())));
                }
            }
        });
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        on_message.forget();
        Ok(())
    }
}

#[wasm_bindgen]
impl EozinViewer {
    #[wasm_bindgen]
    pub async fn start(&mut self) -> Result<(), JsError> {
        if self.is_started || self.receiver.is_none() {
            return Ok(());
        }
        let f: Rc<RefCell<Option<LoopClosure>>> = Rc::new(RefCell::new(None));
        let g = f.clone();

        let (m, workers, sdr, mut rec) = (
            self.model.clone(),
            self.workers.clone(),
            self.sender.clone(),
            self.receiver.take().unwrap(),
        );
        let num_workers = workers.len();
        let context = EozinViewerBuilder::get_canvas(&self.canvas_id).map(|c| {
            c.get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<CanvasRenderingContext2d>()
                .unwrap()
        })?;
        let tick = Some(create_raf_closure(move |perf: f64| {
            let mut w_i = 0;
            let _ = sdr.unbounded_send(tick(perf));
            // m.borrow_mut().tick(perf);
            while let Ok(msg) = rec.try_recv() {
                //console::log_1(&format!("Msg {:?}", msg).into());
                // let cmd = update(&mut m.borrow_mut(), msg);
                let cmd = m.borrow_mut().update(msg);
                if let Cmd::ReadTile(tile_key) = cmd {
                    // console::log_1(&format!("Read Tile-key {:?}", &tile_key).into());
                    let cmd = WorkerMsg::ReadTile {
                        lv: tile_key.lv,
                        x: tile_key.x,
                        y: tile_key.y,
                    }
                    .to_js_value();
                    let _ = workers.get(w_i).map(|w| w.post_message(&cmd));
                    w_i = (w_i + 1) % num_workers;
                }
            }
            let _ = m.borrow_mut().view(&context);
            let _ = request_animation_frame(f.borrow().as_ref().unwrap());
        }));
        *g.borrow_mut() = tick;
        let _ = request_animation_frame(g.borrow().as_ref().unwrap());

        Ok(())
    }
}

fn request_animation_frame(callback: &LoopClosure) -> Result<i32, JsValue> {
    let window = web_sys::window().unwrap();
    window.request_animation_frame(callback.as_ref().unchecked_ref())
}

fn create_raf_closure(f: impl FnMut(f64) + 'static) -> LoopClosure {
    Closure::new(f)
}

/// Message to interact to web workers
///
/// Equivalent JS code is like as following:
/// ```js
/// export type Msg = InitWithFile | ReadTile;
/// interface InitWithFile {
///   type: "InitWithFile";
///   file: File;
/// }
/// interface ReadTile {
///   type: "ReadTile";
///   x: number;
///   y: number;
///   lv: number;
/// }
/// ```
#[derive(Debug)]
pub enum WorkerMsg {
    InitWithFile(File),
    InitWithBlob(Blob),
    ReadTile { lv: usize, x: usize, y: usize },
}

impl WorkerMsg {
    pub fn to_js_value(&self) -> JsValue {
        use WorkerMsg::*;
        match self {
            InitWithFile(file) => {
                let v = js_sys::Object::new().into();
                let _ = js_sys::Reflect::set(&v, &"type".into(), &"InitWithFile".into());
                let _ = js_sys::Reflect::set(&v, &"file".into(), file);
                v
            }
            InitWithBlob(blob) => {
                let v = js_sys::Object::new().into();
                let _ = js_sys::Reflect::set(&v, &"type".into(), &"init_file".into());
                let _ = js_sys::Reflect::set(&v, &"payload".into(), blob);
                v
            }
            ReadTile { x, y, lv } => {
                let v = js_sys::Object::new().into();
                let _ = js_sys::Reflect::set(&v, &"type".into(), &"ReadTile".into());
                let _ = js_sys::Reflect::set(&v, &"x".into(), &(*x as f64).into());
                let _ = js_sys::Reflect::set(&v, &"y".into(), &(*y as f64).into());
                let _ = js_sys::Reflect::set(&v, &"lv".into(), &(*lv as f64).into());
                v
            }
        }
    }
}
