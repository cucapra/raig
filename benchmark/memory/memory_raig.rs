use raig::aiger::run_parser_with_options;
use std::fs::File;
use std::io::BufReader;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: memory_raig <file.aig>");

    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);

    // false = no pre-optimization, for a fair representation comparison
    let graph = run_parser_with_options(&mut reader, false).unwrap();

    std::hint::black_box(graph);
}