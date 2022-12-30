use eozin::aperio;

fn main() {
    let path = "../data/CMU-1.svs";
    // let path = "../data/CMU-1-Small-Region.svs";
    let lv_count = aperio::level_count(path);
    println!("Level count: {:?}", lv_count)
}
