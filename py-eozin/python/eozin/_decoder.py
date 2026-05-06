from __future__ import annotations

import io
from typing import TYPE_CHECKING

from PIL import Image

from ._eozin_rs import EozinPyDecoder as _Decoder

if TYPE_CHECKING:
    from PIL.Image import Image as PILImage


class Eozin(object):
    """
    A high-performance digital pathology (WSI) image decoder implemented in Rust.
    """
    def __init__(self, p: str) -> None:
        """
        Initializes the decoder from the specified file path.

        Args:
            p: Path to the whole-slide image file.
        """
        self._dec = _Decoder(p)

    @property
    def level_count(self) -> int:
        """The total number of resolution levels in the image pyramid."""
        return self._dec.level_count

    @property
    def dimensions(self) -> tuple[int, int]:
        """The (width, height) of the level 0 (highest resolution) image."""
        return self._dec.dimensions

    @property
    def level_dimensions(self) -> list[tuple[int, int]]:
        """A list of (width, height) for each pyramid level."""
        return self._dec.level_dimensions

    @property
    def level_tile_sizes(self) -> list[tuple[int, int]]:
        """The nominal (width, height) of tiles for each level."""
        return self._dec.level_tile_sizes

    def read_tile(self, lv: int, x: int, y: int) -> PILImage:
        """
        Reads a single tile at the specified level and coordinates.

        Args:
            lv: The pyramid level index.
            x: The horizontal tile index.
            y: The vertical tile index.

        Returns:
            The decoded tile as a Pillow Image object.
        """
        b: bytes = self._dec.read_tile_as_bytes(lv, x, y)
        s = io.BytesIO(b)
        img = Image.open(s)
        return img

    def read_region(
        self, 
        location: tuple[int, int], 
        level: int, 
        size: tuple[int, int]
    ) -> PILImage:
        """
        Reads and combines tiles to return a specific region of the slide.

        Args:
            location: (x, y) coordinates of the top-left corner (Level 0).
            level: The pyramid level to read from.
            size: (width, height) of the region to extract.

        Returns:
            A combined RGB Image object representing the region.
        """
        x, y = location
        w, h = size
        tw, th = self.level_tile_sizes[level]
        max_w, max_h = self.level_dimensions[level]

        x0, y0 = max(0, x), max(0, y)
        x1, y1 = min(max_w, x + w), min(max_h, y + h)

        i0, i1 = diva(x0, tw), diva(x1, tw)
        j0, j1 = diva(y0, th), diva(y1, th)

        dx0, dy0 = i0 * tw - x0, j0 * th - y0

        img = Image.new("RGB", size, "white")

        for j in range(j0, j1+1):
            for i in range(i0, i1+1):
                tile = self.read_tile(level, i, j)
                crop_box = [0, 0, tw, th]
                if x == x0:
                    crop_box[0] = dx0
                if y == y0:
                    crop_box[1] = dy0
                if x == x1:
                    crop_box[2] = dx1
                if y == y1:
                    crop_box[3] = dy1
                if crop_box != [0, 0, tw, th]:
                    tile = tile.crop(crop_box)
                    img.paste(tile, (dx0 + i*tw, dy0 + j*th))
                else:
                    img.paste(tile, (dx0 + i*tw, dy0 + j*th))
        return img


def diva(l: int, t: int) -> int:
    if l % t == 0:
        return l // t
    else:
        return 1 + l // t
