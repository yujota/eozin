# Eozin JavaScript: <small>The *dye-namic solution* for digital pathology</small>

Eozin is a pure-Rust decoder library for digital pathology. 
The library provides high-performance, efficient access to individual tiles within large pathology images, enabling seamless integration into web and server-side workflows.
The name is inspired by **eosin**, a fundamental dye used in medical diagnosis.

## Packages

Depending on your environment, use the appropriate package:

- **[@eozin/eozin-web](https://www.npmjs.com/package/@eozin/eozin-web)**: Optimized for browser environments using WebAssembly.
- **[@eozin/eozin-node](https://www.npmjs.com/package/@eozin/eozin-node)**: Tailored for Node.js and Bun with native performance.

---

## @eozin/eozin-web (Browser)

This package enables client-side decoding of digital pathology images directly in the browser.

The following example demonstrates how to load a slide from a URL and render a specific tile on the page using the camelCase API.

```javascript
import init, { Eozin } from "@eozin/eozin-web";

// 1. Initialize the WebAssembly module
await init();

// 2. Fetch slide data as a Blob
const response = await fetch("./some/slide.svs");
const blob = await response.blob();

// 3. Open the slide
const slide = await Eozin.with_blob(blob);

// 4. Display dimensions
const dim = slide.dimensions;
const info = document.createElement("p");
info.textContent = `Slide dimensions: ${dim.width} x ${dim.height} pixels.`;
document.body.appendChild(info);

// 5. Read and render a specific tile (Level 0, X: 5, Y: 6)
const tile = await slide.readTile(0, 5, 6);
const tileBlob = tile.toBlob();

const img = new Image();
img.src = URL.createObjectURL(tileBlob);
img.alt = "Pathology Slide Tile";
document.body.appendChild(img);
```

## @eozin/eozin-node (Node.js / Bun)

For server-side applications, `@eozin/eozin-node` provides direct access to the local file system for optimal performance.

```javascript
import { Eozin } from '@eozin/eozin-node';

// 1. Open the slide.
let slide = await Eozin.withPath("/some/slide.svs");
console.log(slide.dimensions);
console.log(slide.slideFormat);

// 2. Read a specific tile (Level 0, X: 5, Y: 6).
let tile = await slide.readTile(0, 10, 10);
assert.strictEqual(tile.mimeType, "image/jpeg");

// 3. Save tile buffer to a file.
let buf = tile.toUint8Array();
fs.writeFileSync("tile_lv0_x10_y10.jpeg", tile.toUint8Array());
```

## Notes
Vendor and file format names mentioned in this library are the property of their respective owners.
This library is not certified for clinical use.
