# eozinpy

Python bindings for eozin.
Note: This project is only tested on Ubuntu 22.04.

## How to install on Ubuntu 22.04

Install python virtualenv and build essentials on Ubuntu

```sh
$ sudo apt update && sudo apt upgrade
$ sudo apt install python3-venv build-essential
```

Install rust compiler
See [rust-lang.org/tools/install](https://www.rust-lang.org/tools/install) for details.

```sh
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
$ source "$HOME/.cargo/env"
```

Create venv and install eozin

```sh
$ python3 -m venv env
$ source env/bin/activate
$ pip install -e .
```


## Python APIs

Eozin `read_region` method, and properties such as `level_count`, `level_dimensions`, and `dimensions` are intended to mimic [OpenSlide](https://openslide.org/api/python/)'s methods and properties.

```Python
class Eozin:
    level_count: int
    level_dimensions: list[tuple[int, int]]
    dimensions: tuple[int, int]
    level_tile_sizes: list[tuple[int, int]]

    def __new__(path: str) -> Eozin:
    def read_region(location: tuple[int, int], level: int, size: tuple[int, int]) -> PIL.Image.Image:
    def read_tile(level: int, x: int, y: int) -> PIL.Image.Image:
```

### Usages

```Python
from eozinpy import Eozin

e = Eozin("/path/to/digital_pathology/file")
print(e.level_count)
location = (100, 200)
level = 0
size = (200, 150)
img = e.read_region(location, level, size)
img.save()  # img is an instance of Pillow Image
```
