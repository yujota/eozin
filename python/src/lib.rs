use eozin::sync::reader;
use pyo3::prelude::*;
use pyo3::pycell::PyRefMut;
use pyo3::types::PyBytes;

#[pyclass]
struct Aperio {
    data: reader::Aperio,
    #[pyo3(get)]
    level_count: u64,
    #[pyo3(get)]
    level_dimensions: Vec<(u64, u64)>,
    #[pyo3(get)]
    dimensions: (u64, u64),
}
const CODE: &str = r#"
from PIL import Image
import io

def pillow_img(b):
    stream = io.BytesIO(b)
    return Image.open(stream)
"#;

#[pymethods]
impl Aperio {
    #[new]
    fn py_new(path: &str) -> PyResult<Self> {
        let data = reader::Aperio::open(path).unwrap();
        Ok(Aperio {
            level_count: data.level_count.clone(),
            level_dimensions: data.level_dimensions.clone(),
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
        let buf = self_.data.read_tile(level, x, y).unwrap();
        let buf = match buf {
            reader::Tile::Jpeg(b) => b,
            reader::Tile::Jp2k(b) => b,
            _ => panic!("Unknown image data"),
        };
        let py = self_.py();
        let m = PyModule::from_code(py, CODE, "", "")?;
        let f = m.getattr("pillow_img")?;
        let bytes = PyBytes::new(py, &buf);
        let r = f.call1((bytes,))?;
        Ok(r.to_object(py))
    }

    /*
    fn read_region(
        mut self_: PyRefMut<'_, Self>,
        location: (usize, usize),
        level: usize,
        size: (usize, usize),
    ) -> PyResult<PyObject> {
    TODO
    }
    */
}

#[pymodule]
fn eozinpy(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    let _img = PyModule::import(_py, "PIL")?;
    m.add_class::<Aperio>()?;
    Ok(())
}
