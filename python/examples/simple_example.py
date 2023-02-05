from eozinpy import Eozin


def main():
    e = Eozin("/home/ubuntu/data/CMU-1.svs")
    print("level count", e.level_count)
    print("level dimensions", e.level_dimensions)
    print("dimensions", e.dimensions)
    region = e.read_region((800,900), 2, (100, 100))
    region.save("region.jpeg")


if __name__ == "__main__":
    main()
