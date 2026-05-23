# Eozin: <small>The *dye-namic solution* for digital pathology</small>

[![PyPI version](https://img.shields.io/pypi/v/eozin.svg?style=flat-square&logo=python&logoColor=white)](https://pypi.org/project/eozin/)
[![crates.io version](https://img.shields.io/crates/v/eozin.svg?style=flat-square&logo=rust&logoColor=white)](https://crates.io/crates/eozin)
[![npm version web](https://img.shields.io/npm/v/%40eozin/eozin-web.svg?style=flat-square&logo=npm&logoColor=white&label=npm&3Aeozin-web)](https://www.npmjs.com/package/@eozin/eozin-web)
[![npm version node](https://img.shields.io/npm/v/%40eozin/eozin-node.svg?style=flat-square&logo=npm&logoColor=white&label=npm%3Aeozin-node)](https://www.npmjs.com/package/@eozin/eozin-node)

[![Python Docs](https://img.shields.io/badge/docs-Python-blue.svg?style=flat-square&logo=mdbook&logoColor=white)](https://eozin.org/docs/python/)
[![JS Docs](https://img.shields.io/badge/docs-JavaScript-yellow.svg?style=flat-square&logo=mdbook&logoColor=black)](https://eozin.org/docs/js/)
[![Rust Docs](https://img.shields.io/badge/docs-Rust-black.svg?style=flat-square&logo=rust&logoColor=white)](https://docs.rs/eozin/latest/eozin/)

[![Live Demo](https://img.shields.io/badge/Live%20Demo-blue.svg)](https://eozin.org/)

Eozin is a pure-Rust decoder library for digital pathology.
The library's primary purpose is to provide efficient access to individual tiles within 
digital pathology images.
It provides a native Rust API and bindings targeting Python, Web-based JavaScript, and Node.js.
The name is derived from eosin, an essential dye solution used in pathological diagnosis.


## Quickstart

This example demonstrates how to select the lowest resolution level from a digital 
pathology image in a native Rust environment, retrieve the central tile within that level, 
and save it as a JPEG file.

```rust
use eozin::std_io::DynamicDecoder;

let file = std::fs::File::open("/some/slide.svs").unwrap();
let reader = std::io::BufReader::new(file);
let mut decoder = DynamicDecoder::new(reader).unwrap();

let (width, height) = decoder.dimensions();

// Get the index of the lowest resolution level
let lowest_resolution_level = decoder.level_count() - 1;

// level_tile_ranges returns Vec<(horizontal_tiles, vertical_tiles)>
let lowres_tile_ranges = decoder.level_tile_ranges()[lowest_resolution_level];

// Retrieve the tile at the center of the level
let lowres_centered_tile = decoder.read_tile(
  lowest_resolution_level, 
  lowres_tile_ranges.0 / 2, 
  lowres_tile_ranges.1 / 2, 
).unwrap();

assert!(lowres_centered_tile.is_jpeg());
std::fs::write("tile.jpg", &lowres_centered_tile).unwrap();
```

## Core Concept

Digital pathology images are captured using high-magnification microscopes, 
resulting in massive images that can reach hundreds of thousands of pixels in dimension.
Because storing such large images in standard formats is impractical, 
they are designed as containers comprising small rectangular images called **tiles**.

Furthermore, since downscaling these images on the fly is computationally expensive, 
they typically store multiple lower-resolution versions, referred to as **levels**. 
Vendors have developed various proprietary formats, and libraries like OpenSlide 
and bioformats have been instrumental in handling them.

The motivation for Eozin is to provide lightweight and fast access to these tiles. 
Specifically, each tile is stored as a fragmented byte sequence; Eozin calculates 
the byte offset based on the requested level and coordinates, then appends 
the necessary headers so the buffer can be interpreted as a standard image format.

Pixel-level processing (such as intensity manipulation) is intentionally left 
to established libraries—such as the `image` crate in Rust, `Pillow` in Python, 
or `Blob` objects in JS/Web environments.

Since the I/O and decoding logic are completely decoupled, Eozin can be adapted to 
various environments. Client-side rendering in the browser achieves performant 
response times, and its efficiency is also ideal for high-throughput AI 
interpretation and analysis.


## Notes
Vendor and file format names mentioned in this library are the property of their respective owners.
This library is not certified for clinical use.
