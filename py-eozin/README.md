# Eozin Python: <small>The *dye-namic solution* for digital pathology</small>

A fast digital pathology image decoder powered by Rust.
The library's primary purpose is to provide efficient access to individual tiles within 
digital pathology images.
The name is derived from eosin, an essential dye solution used in pathological diagnosis.


## Quickstart

This example demonstrates how to select the lowest resolution level from a digital 
pathology image in a native Rust environment, retrieve the central tile within that level, 
and save it as a JPEG file.

```python
from eozin import Eozin
from PIL import Image

# Load a slide image
slide = Eozin("/some/slide.svs")

# Get dimensions of the slide at level 0 (full resolution)
slide_width, slide_height = slide.dimensions

# Get the index of the lowest resolution level
lowest_resolution_level = slide.level_count - 1

# level_tile_ranges returns a list of tuples:
# [(horizontal_tiles, vertical_tiles), ...]
lowres_tile_ranges = slide.level_tile_ranges[lowest_resolution_level]

# Retrieve the tile at the center of the level
# lowres_tile_ranges[0] is the width in tiles, and [1] is the height in tiles.
lowres_centered_tile: Image.Image = slide.read_tile(
  lowest_resolution_level, 
  lowres_tile_ranges[0] // 2, 
  lowres_tile_ranges[1] // 2, 
)
lowres_centered_tile.show()

# Extract a specific region (OpenSlide-like API)
# read_region(location, level, size)
region: Image.Image = slide.read_region((0, 0), 0, (1024, 1024))
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
