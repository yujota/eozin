use wasm_bindgen::prelude::*;

use eozin;

#[wasm_bindgen]
pub async fn level_count(blob: web_sys::Blob) -> Result<u64, JsError> {
    eozin::wasm::level_count(blob).await.map_err(|e| e.into())
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
