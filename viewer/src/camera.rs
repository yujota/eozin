pub(crate) type Lv0Pos = (f64, f64);
pub(crate) type ZoomFactor = f64;

#[derive(Debug, Clone)]
pub(crate) struct CameraBuilder {
    exp_delta: f64,  // Default 0.25
    exp_uplim: f64,  // Default +2
    exp_lowlim: f64, // Default -2
}

impl CameraBuilder {
    pub(crate) fn new() -> CameraBuilder {
        CameraBuilder {
            exp_delta: 0.25,
            exp_uplim: 2.0,
            exp_lowlim: -2.0,
        }
    }
    pub(crate) fn build(self, canvas_size: (f64, f64), lv0_size: (f64, f64)) -> Camera {
        let lv0_width = lv0_size.0;
        let lv0_height = lv0_size.1;
        let canvas_width = canvas_size.0;
        let canvas_height = canvas_size.1;
        let r = (canvas_width / lv0_width).min(canvas_height / lv0_height);
        let r_exp = -r.log2();
        let max_zoom_exp = (r_exp / self.exp_delta).floor() * self.exp_delta + self.exp_uplim;
        Camera {
            canvas_height,
            canvas_width,
            zoom_exp: 0.0,
            lv0_width,
            lv0_height,
            x: 0.0,
            y: 0.0,
            min_zoom_exp: self.exp_lowlim,
            max_zoom_exp,
            moving: None,
            exp_delta: self.exp_delta,
            is_updated: false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Camera {
    canvas_width: f64,
    canvas_height: f64,
    lv0_width: f64,
    lv0_height: f64,
    // zoom_factor: ZoomFactor,
    zoom_exp: f64,  // negative zoom exponet
    exp_delta: f64, // negative zoom exponet
    x: f64,         // Lv0Pos
    y: f64,
    min_zoom_exp: f64,
    max_zoom_exp: f64,
    moving: Option<((f64, f64), Lv0Pos)>,
    is_updated: bool,
}

#[allow(dead_code)]
impl Camera {
    pub(crate) fn move_start(&mut self, pointer_x: f64, pointer_y: f64) {
        self.moving = Some(((pointer_x, pointer_y), (self.x, self.y)));
    }
    pub(crate) fn update_move(&mut self, pointer_x: f64, pointer_y: f64) {
        if let Some(((px, py), (lx, ly))) = self.moving {
            let r = self.zoom_factor();
            let new_x = -r * (pointer_x - px) + lx;
            let new_y = -r * (pointer_y - py) + ly;
            if self.in_boundary(new_x, new_y) {
                self.x = new_x;
                self.y = new_y;
                self.is_updated = true;
            }
        }
    }
    pub(crate) fn update_canvas_size(&mut self, width: f64, height: f64) {
        self.canvas_height = height;
        self.canvas_width = width;
    }
    pub(crate) fn zoom_out(&mut self, client_x: f64, client_y: f64) {
        let next_zoom_exp = self.zoom_exp + self.exp_delta;
        if next_zoom_exp > self.max_zoom_exp {
            return;
        }
        let next_zoom_exp = (next_zoom_exp / self.exp_delta).floor() * self.exp_delta;
        self.zoom(client_x, client_y, next_zoom_exp);
    }
    pub(crate) fn zoom_in(&mut self, client_x: f64, client_y: f64) {
        let next_zoom_exp = self.zoom_exp - self.exp_delta;
        if next_zoom_exp < self.min_zoom_exp {
            return;
        }
        let next_zoom_exp = (next_zoom_exp / self.exp_delta).floor() * self.exp_delta;
        self.zoom(client_x, client_y, next_zoom_exp);
    }
    pub(crate) fn zoom_with(&mut self, client_x: f64, client_y: f64, zoom_exp: f64) {
        if zoom_exp < self.min_zoom_exp || zoom_exp > self.max_zoom_exp {
            return;
        }
        self.zoom(client_x, client_y, zoom_exp);
    }

    fn zoom(&mut self, client_x: f64, client_y: f64, next_zoom_exp: f64) {
        // web_sys::console::log_1(&format!("Zoom With {:?}, {}", &client_x, &client_y).into());
        let r = 2.0_f64.powf(next_zoom_exp);
        let zf = self.zoom_factor();
        let dx = client_x * (zf - r);
        let dy = client_y * (zf - r);
        self.x += dx;
        self.y += dy;
        self.zoom_exp = next_zoom_exp;
        self.is_updated = true;
    }
    pub(crate) fn top_left(&self) -> Lv0Pos {
        (self.x, self.y)
    }
    pub(crate) fn bottom_right(&self) -> Lv0Pos {
        let (w, h) = self.lv0_size();
        (self.x + w, self.y + h)
    }
    pub(crate) fn center(&self) -> Lv0Pos {
        let half_w = self.canvas_width / 2.0;
        let half_h = self.canvas_height / 2.0;
        let r = 2.0_f64.powf(-self.zoom_exp);
        (self.x + half_w * r, self.y + half_h * r)
    }

    pub(crate) fn to_lv0_pos(&self, pointer_x: f64, pointer_y: f64) -> Lv0Pos {
        let r = 2.0_f64.powf(-self.zoom_exp);
        (self.x + pointer_x * r, self.y + pointer_y * r)
    }

    pub(crate) fn from_lv0_pos(&self, lv0_x: f64, lv0_y: f64) -> (f64, f64) {
        let r = 2.0_f64.powf(self.zoom_exp);
        ((lv0_x - self.x) * r, (lv0_y - self.y) * r)
    }

    // Camera size
    pub(crate) fn lv0_size(&self) -> (f64, f64) {
        let r = 2.0_f64.powf(self.zoom_exp);
        (self.canvas_width * r, self.canvas_height * r)
    }
    // Fit Camera position and magnification to capture whole area of slide
    pub(crate) fn fit_to_frame(&mut self) {
        // println!("R {}, {}", self.canvas_width, self.lv0_width);
        // println!("R {}, {}", self.canvas_height, self.lv0_height);
        let r = (self.canvas_width / self.lv0_width).min(self.canvas_height / self.lv0_height);
        // println!("R {}", r);
        let r_exp = -r.log2();
        self.zoom_exp = r_exp;
        let half_w = self.canvas_width / 2.0;
        let half_h = self.canvas_height / 2.0;
        let new_x = half_w / r - self.lv0_width * 0.5;
        let new_y = half_h / r - self.lv0_height * 0.5;
        self.x = -new_x;
        self.y = -new_y;
    }

    fn in_boundary(&self, x: f64, y: f64) -> bool {
        let zf = self.zoom_factor();
        let cw = self.canvas_width * zf;
        let ch = self.canvas_height * zf;
        if x < -cw || y < -ch {
            return false;
        };
        x < self.lv0_width + cw && y < self.lv0_height + ch
    }

    // When camera's magnicication is same as Lv0, zoom factor shold be 1.
    // When camera's magnicication is same as Lv1 whose length is a half of
    // lv0 width, zoom factor shold be 2.
    pub(crate) fn zoom_factor(&self) -> ZoomFactor {
        2.0_f64.powf(self.zoom_exp)
    }

    pub(crate) fn zoom_exp(&self) -> f64 {
        self.zoom_exp
    }

    pub(crate) fn applied(&mut self) {
        self.is_updated = false;
    }

    pub(crate) fn is_updated(&self) -> bool {
        self.is_updated
    }
    pub(crate) fn canvas_size(&self) -> (f64, f64) {
        (self.canvas_width, self.canvas_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zoom_factor() {
        let canvas_size = (1200.0, 800.0);
        let lv0_size = (1200.0, 800.0);
        let camera = CameraBuilder::new().build(canvas_size, lv0_size);
        assert_eq!(1.0, camera.zoom_factor());
    }

    #[test]
    fn test_move() {
        let canvas_size = (1600.0, 800.0);
        let lv0_size = (1200.0, 800.0);
        let mut camera = CameraBuilder::new().build(canvas_size, lv0_size);
        camera.move_start(0.0, 0.0);
        camera.update_move(400.0, 0.0);
        assert_eq!((-400.0, 0.0), camera.top_left());
    }

    #[test]
    fn test_fit_frame() {
        let canvas_size = (1200.0, 800.0);
        let lv0_size = (1200.0, 800.0);
        let mut camera = CameraBuilder::new().build(canvas_size, lv0_size);
        camera.fit_to_frame();
        assert_eq!(1.0, camera.zoom_factor());
        assert_eq!((0.0, 0.0), camera.top_left());
    }

    #[test]
    fn test_zoom_out() {
        let canvas_size = (1200.0, 800.0);
        let lv0_size = (1200.0, 800.0);
        let mut camera = CameraBuilder::new().build(canvas_size, lv0_size);
        camera.zoom_out(600.0, 400.0);
        camera.zoom_out(600.0, 400.0);
        camera.zoom_out(600.0, 400.0);
        camera.zoom_out(600.0, 400.0);
        assert_eq!(2.0_f64.powf(1.0), camera.zoom_factor());
        assert_eq!((-600.0, -400.0), camera.top_left());
    }
}
