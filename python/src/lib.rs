use ::std::env::current_exe;

use eozin::std;
use num::integer::div_mod_floor;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::pycell::PyRefMut;
use pyo3::types::PyBytes;

#[pyclass]
struct Eozin {
    data: std::Eozin,
    #[pyo3(get)]
    level_count: u64,
    #[pyo3(get)]
    level_dimensions: Vec<(u64, u64)>,
    #[pyo3(get)]
    dimensions: (u64, u64),
    #[pyo3(get)]
    level_tile_sizes: Vec<(u64, u64)>,
}
const PYCODE_READ_TILE: &str = r#"
from PIL import Image
import io

def pillow_img(b):
    stream = io.BytesIO(b)
    return Image.open(stream)
"#;

const PYCODE_READ_REGION: &str = r#"
from PIL import Image
import io

def pillow_img(size, imgs):
    result = Image.new("RGB", size, "white")
    for (b, crop_box, pos) in imgs:
        stream = io.BytesIO(b)
        img = Image.open(stream)
        if crop_box == (0, 0, 0, 0):
            result.paste(img, pos)
        else:
            img = img.crop(crop_box)
            result.paste(img, pos)
    return result
"#;

#[pymethods]
impl Eozin {
    #[new]
    fn py_new(path: &str) -> PyResult<Self> {
        let data = std::Eozin::open(path).unwrap();
        Ok(Eozin {
            level_count: data.level_count.clone(),
            level_dimensions: data.level_dimensions.clone(),
            level_tile_sizes: data.level_tile_sizes.clone(),
            dimensions: data.dimensions.clone(),
            data,
        })
    }

    fn read_tile(
        mut self_: PyRefMut<'_, Self>,
        level: usize,
        x: usize,
        y: usize,
    ) -> PyResult<PyObject> {
        let buf = self_
            .data
            .read_tile(level, x, y)
            .map(|b| b.buffer().clone())
            .unwrap();
        let py = self_.py();
        let m = PyModule::from_code(py, PYCODE_READ_TILE, "", "")?;
        let f = m.getattr("pillow_img")?;
        let bytes = PyBytes::new(py, &buf);
        let r = f.call1((bytes,))?;
        Ok(r.to_object(py))
    }

    fn read_region(
        mut self_: PyRefMut<'_, Self>,
        location: (usize, usize),
        level: usize,
        size: (usize, usize),
    ) -> PyResult<PyObject> {
        let (tw, th) = self_.level_tile_sizes.get(level).unwrap();
        let (img_w, img_h) = self_.level_dimensions.get(level).unwrap();
        let (tw, th, img_w, img_h) = (*tw, *th, *img_w, *img_h);
        let (x0, y0) = (location.0 as u64, location.1 as u64);
        let (x1, y1) = (x0 + size.0 as u64, y0 + size.1 as u64);

        if !(x0 <= img_w && y0 <= img_h && x1 <= img_w && y1 <= img_h) {
            return Err(PyValueError::new_err("Out of index"));
        }

        let helper = RegionHelper::new(tw, th, x0, x1, y0, y1);
        let query = helper.query();
        let query: Vec<(Vec<u8>, (u64, u64, u64, u64), (u64, u64))> = query
            .into_iter()
            .map(|((i, j), c_box, pos)| {
                (
                    self_
                        .data
                        .read_tile(level, i as usize, j as usize)
                        .map(|b| b.buffer().clone())
                        .unwrap(),
                    c_box.unwrap_or((0, 0, 0, 0)),
                    pos,
                )
            })
            .collect();

        let py = self_.py();
        let m = PyModule::from_code(py, PYCODE_READ_REGION, "", "")?;
        let f = m.getattr("pillow_img")?;
        let query: Vec<(&PyBytes, (u64, u64, u64, u64), (u64, u64))> = query
            .iter()
            .map(|(b, c, p)| (PyBytes::new(py, b), *c, *p))
            .collect();
        let r = f.call((size, query), None)?;
        Ok(r.to_object(py))
    }
}

struct RegionHelper {
    tw: u64,
    th: u64,

    i0: u64,
    i1: u64,
    j0: u64,
    j1: u64,

    dx0: u64,
    dx1: u64,
    dy0: u64,
    dy1: u64,
}

impl RegionHelper {
    fn new(tw: u64, th: u64, x0: u64, x1: u64, y0: u64, y1: u64) -> Self {
        // tw: tile width, th: tile height
        // (x0, y0): top left point's coordinate
        // (x1, y1): bottom right point's coordinate
        // (i0, j0): top left point's tile index
        // (dx0, dy0): diff from top left tile's coordinate to the point
        let ((i0, dx0), (j0, dy0)) = (div_mod_floor(x0, tw), div_mod_floor(y0, th));
        let ((i1, dx1), (j1, dy1)) = (div_mod_floor(x1, tw), div_mod_floor(y1, th));
        Self {
            tw,
            th,
            i0,
            i1,
            j0,
            j1,
            dx0,
            dx1,
            dy0,
            dy1,
        }
    }

    fn query(&self) -> Vec<((u64, u64), Option<(u64, u64, u64, u64)>, (u64, u64))> {
        let i_max = if self.dx1 == 0 { self.i1 } else { self.i1 + 1 };
        let j_max = if self.dy1 == 0 { self.j1 } else { self.j1 + 1 };

        let mut qs = Vec::new();

        for j in self.j0..j_max {
            for i in self.i0..i_max {
                let crop_box = if self.i0 < i && i < self.i1 && self.j0 < j && j < self.j1 {
                    None
                } else {
                    let left = if self.i0 == i { self.dx0 } else { 0 };
                    let right = if i == self.i1 { self.dx1 } else { self.tw };
                    let top = if self.j0 == j { self.dy0 } else { 0 };
                    let bottom = if j == self.j1 { self.dy1 } else { self.th };
                    Some((left, top, right, bottom))
                };
                let x = if self.i0 == i {
                    0
                } else {
                    self.tw * (i - self.i0) - self.dx0
                };
                let y = if self.j0 == j {
                    0
                } else {
                    self.th * (j - self.j0) - self.dy0
                };
                qs.push(((i, j), crop_box, (x, y)));
            }
        }
        qs
    }
}

#[pymodule]
fn eozinpy(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Eozin>()?;
    Ok(())
}

fn read_region_helper() {}
