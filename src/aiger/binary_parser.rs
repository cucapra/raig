use std::io::{BufRead, Error, Read};

use crate::aiger::{AigerHeader, LineReader, Literals};
use crate::graph::{AigBuilder, AigGraph, NodeId};

pub fn parse_binary_aiger_into_graph(
    header: AigerHeader,
    reader: &mut impl BufRead,
    pre_optimize: bool,
) -> Result<AigGraph, Error> {
    let mut graph = AigBuilder::new();
    let mut literals = Literals::new(header.max_var);

    for input_index in 0..header.num_inputs {
        let input_lit: usize = 2 * (input_index + 1);
        let input_id: NodeId = graph.add_input();
        literals.add(input_lit, input_id);
    }

    // Parse the ASCII-formatted lines.
    let mut latch_inputs: Vec<(NodeId, usize)> = Vec::with_capacity(header.num_inputs);
    // outputs and the different asserts/assumptions are all essentially semantic labels for expressions
    let mut label_lits: Vec<usize> = Vec::with_capacity(header.num_labels());
    {
        let mut line_reader = LineReader::new(reader);

        // like in ascii parser, save latches for later
        for latch_index in 0..header.num_latches {
            let latch_input_lit = line_reader.read_int()?.expect("malformed latch literal");
            let latch_lit: usize = 2 * (header.num_inputs + latch_index + 1);
            let latch_id: NodeId = graph.add_latch(NodeId::FALSE);

            literals.add(latch_lit, latch_id);
            latch_inputs.push((latch_id, latch_input_lit));
        }
        // same idea for labels
        for _ in 0..header.num_labels() {
            let lit = line_reader.read_int()?.expect("malformed output literal");
            label_lits.push(lit);
        }
    }

    for and_index in 0..header.num_and_gates {
        let lhs_lit: usize = 2 * (header.num_inputs + header.num_latches + and_index + 1);
        let delta0: usize = read_delta(reader)?;
        let delta1: usize = read_delta(reader)?;

        let rhs0_lit: usize = lhs_lit - delta0;
        let rhs1_lit: usize = rhs0_lit - delta1;

        let left: NodeId = literals.get(rhs0_lit);
        let right: NodeId = literals.get(rhs1_lit);

        let and_id = if pre_optimize {
            graph.add_and_optimized(left, right)
        } else {
            graph.add_and_raw(left, right)
        };

        literals.add(lhs_lit, and_id);
    }

    // resolve latches
    for (latch_id, latch_input_lit) in latch_inputs {
        let latch_input_id: NodeId = literals.get(latch_input_lit);
        graph.node(latch_id).set_latch_input(latch_input_id);
    }

    // resolve labels
    label_lits.reverse();
    for _ in 0..header.num_outputs {
        graph.add_output(literals.get(label_lits.pop().unwrap()));
    }
    for _ in 0..header.num_bad_states {
        graph.add_bad_state(literals.get(label_lits.pop().unwrap()));
    }
    for _ in 0..header.num_invariants {
        graph.add_invariant(literals.get(label_lits.pop().unwrap()));
    }
    for _ in 0..header.num_justice {
        graph.add_justice(literals.get(label_lits.pop().unwrap()));
    }
    for _ in 0..header.num_fairness {
        graph.add_fairness(literals.get(label_lits.pop().unwrap()));
    }
    debug_assert!(label_lits.is_empty());


    Ok(graph.build())
}

/// Decodes the binary-encoded AND gate representation.
///
/// In binary AIGER, each AND gate is stored using two deltas values, where:
///
///
/// delta0 = lhs  - rhs0
/// delta1 = rhs0 - rhs1
///
/// Given `lhs`, these deltas let us recover:
///
///
/// rhs0 = lhs  - delta0
/// rhs1 = rhs0 - delta1
///
///
/// Each delta is encoded as a variable-length little-endian integer.
/// In each byte, the most significant bit is a continuation bit:
///
/// - `1` means the integer continues in the next byte.
/// - `0` means this byte is the last byte of the current integer.
///
/// Therefore, the decoder first reads bytes until it finishes `delta0`,
/// then reads bytes until it finishes `delta1` (on the second call to the function, resuing the same reader)
///
/// AIGER requires the ordering:
///
///
/// lhs > rhs0 >= rhs1
///
///
/// This guarantees that both deltas are nonnegative. In practice, the
/// deltas are usually small, which makes the encoding nice and compact!
fn read_delta(reader: &mut impl Read) -> Result<usize, Error> {
    let mut value: usize = 0usize;
    let mut shift: u32 = 0u32;

    loop {
        let mut byte: [u8; 1] = [0u8; 1];
        reader.read_exact(&mut byte)?;

        // removes the top bit and keeps only the lower 7 bits
        let chunk: usize = (byte[0] & 0b01111111) as usize;

        // inserts that 7-bit chunk into the correct place in the final number
        value |= chunk << shift;

        if (byte[0] & 0b10000000) == 0 {
            return Ok(value);
        }

        shift += 7;
    }
}
