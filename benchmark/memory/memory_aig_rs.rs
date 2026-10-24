use aig::Aig;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: memory_aig_rs <file.aig>");

    let graph = Aig::from_file(path);

    std::hint::black_box(graph);
}