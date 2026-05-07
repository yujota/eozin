#[cfg(all(feature = "node", not(feature = "web")))]
pub use eozin::wasm::node::DynamicDecoder;

#[cfg(all(feature = "web", not(feature = "node")))]
pub use eozin::wasm::web::DynamicDecoder;
