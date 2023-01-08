use eozin::aperio;
use eozin::reader;

fn aperio() {
    let path = "../data/CMU-1.svs";
    // let path = "../data/CMU-1-Small-Region.svs";
    let lv_count = aperio::level_count(path);
    println!("Level count: {:?}", lv_count)
}

fn leica() {
    let path = "../data/Leica-1.scn";
    let leica_compatible = reader::is_leica(path);
    println!("Leica compat: {:?}", leica_compatible)
}

fn main() {
    leica()
}
