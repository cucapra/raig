use mutaig::Aig;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: memory_mutaig <file.aig>");

    let graph = Aig::from_file(path).unwrap();

    std::hint::black_box(graph);
}