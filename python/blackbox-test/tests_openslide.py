import os
import random
import unittest

import requests
from openslide import OpenSlide
from numpy.testing import assert_array_equal
import numpy as np

import eozinpy


LINK_APERIO = "https://openslide.cs.cmu.edu/download/openslide-testdata/Aperio/"
APERIO_FILES = ( 
    # "CMU-1.svs",
    "CMU-2.svs",
    "CMU-3.svs",
    "JP2K-33003-1.svs",
    "JP2K-33003-2.svs",
)
DATA_DIR = "data"


def download_files():
    data_dir = "data"
    os.makedirs(data_dir, exist_ok=True)
    for f_name in APERIO_FILES:
        f_path = os.path.join(data_dir, f_name)
        if os.path.isfile(f_path):
            continue
        url = f"{LINK_APERIO}{f_name}"
        res = requests.get(url, stream=True)
        assert res.status_code == 200
        with open(f_path, "wb") as f:
            f.write(res.content)



def test_aperio():
    f_path = os.path.join(DATA_DIR, APERIO_FILES[1])
    ez_slide = eozinpy.Eozin(f_path)
    os_slide = OpenSlide(f_path)
    pass


class TestAperio(unittest.TestCase):
    def test_level_dimensions(self):
        for n in APERIO_FILES:
            f_path = os.path.join(DATA_DIR, n)
            assert os.path.isfile(f_path)
            with self.subTest(msg=n):
                test_level_dimensions(self, f_path)

    def test_level_count(self):
        for n in APERIO_FILES:
            f_path = os.path.join(DATA_DIR, n)
            assert os.path.isfile(f_path)
            with self.subTest(msg=n):
                test_level_count(self, f_path)

    def test_dimensions(self):
        for n in APERIO_FILES:
            f_path = os.path.join(DATA_DIR, n)
            assert os.path.isfile(f_path)
            with self.subTest(msg=n):
                test_dimensions(self, f_path)

    def test_read_region(self):
        for n in APERIO_FILES:
            f_path = os.path.join(DATA_DIR, n)
            assert os.path.isfile(f_path)
            with self.subTest(msg=n):
                test_read_region(self, f_path)


def test_level_dimensions(test_case, f_path):
    ez_slide = eozinpy.Eozin(f_path)
    os_slide = OpenSlide(f_path)

    actual = ez_slide.level_dimensions
    expected = os_slide.level_dimensions
    test_case.assertEqual(actual, expected)


def test_level_count(test_case, f_path):
    ez_slide = eozinpy.Eozin(f_path)
    os_slide = OpenSlide(f_path)

    actual = ez_slide.level_count
    expected = os_slide.level_count
    test_case.assertEqual(actual, expected)


def test_dimensions(test_case, f_path):
    ez_slide = eozinpy.Eozin(f_path)
    os_slide = OpenSlide(f_path)

    actual = ez_slide.dimensions
    expected = os_slide.dimensions
    test_case.assertEqual(actual, expected)


def test_read_region(test_case, f_path):
    ez_slide = eozinpy.Eozin(f_path)
    os_slide = OpenSlide(f_path)
    n = os.path.basename(f_path).split(".")[0]

    dims = list(enumerate(os_slide.level_dimensions))
    lv, (w, h) = random.choice(dims)
    x0 = random.randint(0, w - 50)
    y0 = random.randint(0, h - 50)
    x1 = random.randint(0, 50)
    y1 = random.randint(0, 50)
    size = (x1, y1)
    loc = (x0, y0)

    actual = ez_slide.read_region(loc, lv, size)
    actual.save(f"{n}-ez.png")
    expected = os_slide.read_region(loc, lv, size)
    expected.save(f"{n}-os.png")
    act = np.array(actual)
    exp = np.array(expected)[:, :, :3]
    diff = np.maximum(act, exp) - np.minimum(act, exp)
    np.testing.assert_array_less(diff, np.ones(shape=diff.shape) * 25)


def main():
    suite = unittest.TestSuite()
    suite.addTest(unittest.makeSuite(TestAperio))
    runner = unittest.TextTestRunner()
    runner.run(suite)


if __name__ == "__main__":
    main()
