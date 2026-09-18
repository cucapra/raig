use std::io::{BufRead, Error};

use crate::aiger::{
    AigerHeader, LineReader, Literals, parse_symbol_table_and_comments, resolve_labels_and_latches,
};
use crate::graph::{AigBuilder, AigGraph, NodeId, SymbolTable};

pub fn parse_ascii_aiger_into_graph(
    header: AigerHeader,
    reader: &mut impl BufRead,
    pre_optimize: bool,
) -> Result<(AigGraph, SymbolTable, String), Error> {
    let mut graph = AigBuilder::new();
    let mut literals = Literals::new(header.max_var);
    let mut line_reader = LineReader::new(reader);

    for _ in 0..header.num_inputs {
        let input_lit = line_reader.read_int()?.expect("malformed input line");

        let input_id: NodeId = graph.add_input();
        literals.add(input_lit, input_id);
    }

    let mut latch_inputs: Vec<(NodeId, usize)> = Vec::with_capacity(header.num_latches);

    // note: we add Nodeid::FALSE because latches may
    // contain nodes that are not defined yet (ex. AND nodes),
    // so we put them in the graph but save them in a hashmap for later
    for _ in 0..header.num_latches {
        let [latch_lit, latch_input_lit] = line_reader.read_ints()?.expect("malformed latch line");

        let latch_id = graph.add_latch(NodeId::FALSE);
        literals.add(latch_lit, latch_id);
        latch_inputs.push((latch_id, latch_input_lit));
    }

    // same idea for all labels, we save them for later
    let mut label_lits: Vec<usize> = Vec::with_capacity(header.num_labels());

    for _ in 0..header.num_labels() {
        let output_lit = line_reader.read_int()?.expect("malformed output line");
        label_lits.push(output_lit);
    }

    for _ in 0..header.num_and_gates {
        let [lhs_lit, rhs0_lit, rhs1_lit] = line_reader.read_ints()?.expect("malformed and line");

        let left: NodeId = literals.get(rhs0_lit);
        let right: NodeId = literals.get(rhs1_lit);

        let and_id: NodeId = if pre_optimize {
            graph.add_and_optimized(left, right)
        } else {
            graph.add_and_raw(left, right)
        };

        literals.add(lhs_lit, and_id);
    }

    resolve_labels_and_latches(label_lits, latch_inputs, &header, &mut graph, &literals);

    // optional symbol table
    let (st, comments) = parse_symbol_table_and_comments(&mut line_reader)?;
    Ok((graph.build(), st, comments))
}
