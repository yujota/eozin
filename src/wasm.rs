//! WebAssembly.
//! This module is intended to be compiled with wasm-pack and called by JavaScript.
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[cfg(feature = "wasm-node")]
pub mod node;
#[cfg(feature = "wasm-web")]
pub mod web;

/// Represents image dimensions (width and height).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct Dimension {
    pub width: u32,
    pub height: u32,
}

/// Represents the tile grid range (number of horizontal and vertical tiles).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct TileRange {
    pub x: u32,
    pub y: u32,
}

impl From<(u64, u64)> for Dimension {
    fn from((width, height): (u64, u64)) -> Dimension {
        Dimension {
            width: width as u32,
            height: height as u32,
        }
    }
}

impl From<(usize, usize)> for TileRange {
    fn from((x, y): (usize, usize)) -> TileRange {
        TileRange {
            x: x as u32,
            y: y as u32,
        }
    }
}
