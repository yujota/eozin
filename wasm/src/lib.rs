use wasm_bindgen::prelude::*;

use eozin;

#[wasm_bindgen]
pub struct Eozin {
    aperio: eozin::wasm::Aperio,
    pub level_count: u64,
}

#[wasm_bindgen]
impl Eozin {
    #[wasm_bindgen(constructor)]
    pub async fn new(blob: web_sys::Blob) -> Result<Eozin, JsError> {
        let aperio = eozin::wasm::Aperio::open(blob).await?;
        let level_count = aperio.level_count.clone();
        Ok(Eozin {
            aperio,
            level_count,
        })
    }

    #[wasm_bindgen(method)]
    pub async fn read_tile(
        &mut self,
        lv: usize,
        x: usize,
        y: usize,
    ) -> Result<web_sys::Blob, JsError> {
        self.aperio.read_tile(lv, x, y).await.map_err(|e| e.into())
    }
}
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
