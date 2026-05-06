use eozin::std_io::DynamicDecoder;
use pyo3::prelude::*;
use std::fs::File;
use std::io::BufReader;

#[pyclass(unsendable)]
pub struct EozinPyDecoder {
    decoder: DynamicDecoder<BufReader<File>>,
    #[pyo3(get)]
    level_count: u64,
    #[pyo3(get)]
    dimensions: (u64, u64),
    #[pyo3(get)]
    level_dimensions: Vec<(u64, u64)>,
    #[pyo3(get)]
    level_tile_sizes: Vec<(u64, u64)>,
}

#[pymethods]
impl EozinPyDecoder {
    #[new]
    pub fn py_new(path: &str) -> PyResult<Self> {
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let decoder = DynamicDecoder::new(reader).unwrap();
        let dimensions = decoder.dimensions();
        let level_dimensions = decoder.level_dimensions();
        let level_tile_sizes = decoder.level_tile_sizes();
        let level_count = decoder.level_count() as u64;
        Ok(EozinPyDecoder {
            decoder,
            dimensions,
            level_count,
            level_dimensions,
            level_tile_sizes,
        })
    }

    fn read_tile_as_bytes(mut self_: PyRefMut<'_, Self>, lv: usize, x: usize, y: usize) -> Vec<u8> {
        self_.decoder.read_tile_as_bytes(lv, x, y).unwrap()
    }
}
