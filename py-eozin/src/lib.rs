mod decoder;

use pyo3::{prelude::*};

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

#[pymodule]
#[pyo3(name = "_eozin_rs")]
fn eozin(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    // Add the __version__ attribute to the Python module
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    m.add_class::<decoder::EozinPyDecoder>()?;
    Ok(())
}
