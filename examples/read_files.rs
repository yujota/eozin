use eozin::sync::reader as sreader;

fn sync_reader() {
    let path = "../data/CMU-1.svs";
    // let path = "../data/Leica-1.scn";
    sreader::read(path);
}

fn main() {
    sync_reader()
}
