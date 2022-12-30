use eozin::aperio;
use pyo3::prelude::*;

#[pyfunction]
fn level_count(path: &str) -> PyResult<u64> {
    Ok(aperio::level_count(path))
}

#[pymodule]
fn eozinpy(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(level_count, m)?)?;
    Ok(())
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
