use eozin::std as sreader;
use eozin::std::Eozin;
use std::fs::File;
use std::io::{BufWriter, Write};

fn sync_reader() {
    let path = "../data/CMU-1.svs";
    let path = "../data/JP2K-33003-1.svs";
    let path = "../data/CMU-1-JP2K-33005.svs";
    // let path = "../data/CMU-1-Small-Region.svs";
    // let path = "../data/Leica-1.scn";
    // sreader::read(path);

    let mut eozin = Eozin::open(path).unwrap();
    let lv_count = eozin.level_count;
    println!("lv_count: {:?}", lv_count);

    let lv_dimensions = &eozin.level_dimensions;
    println!("lv_dimensions: {:?}", lv_dimensions);

    let tile = eozin.read_tile(0, 20, 15).unwrap();

    let mut output = File::create("tmp.jpeg").unwrap();
    let mut writer = BufWriter::new(&mut output);
    writer.write(tile.buffer()).unwrap();
    writer.flush().unwrap();
}

fn main() {
    sync_reader()
}
