// Copyright 2026 Cornell University
// released under MIT License
// author: Kevin Laeufer <laeufer@cornell.edu>

use crate::aiger::AigerMode;
use crate::graph::{AigGraph, NodeId, SymbolTable};
use std::io::{Result, Write};

pub fn write_ascii_aiger(g: &AigGraph, out: &mut impl Write) -> Result<()> {
    write_aiger(g, AigerMode::Ascii, out)
}

pub fn write_aiger(g: &AigGraph, mode: AigerMode, out: &mut impl Write) -> Result<()> {
    write_aiger_with_symbol_table(g, &SymbolTable::default(), mode, out)
}

pub fn write_aiger_with_symbol_table(
    g: &AigGraph,
    st: &SymbolTable,
    mode: AigerMode,
    out: &mut impl Write,
) -> Result<()> {
    write_aiger_with_symbol_table_and_comments(g, st, &"", mode, out)
}

pub fn write_aiger_with_symbol_table_and_comments<'a>(
    g: &AigGraph,
    st: &SymbolTable,
    comments: &str,
    mode: AigerMode,
    out: &mut impl Write,
) -> Result<()> {
    write_header(g, mode, out)?;
    match mode {
        AigerMode::Ascii => write_ascii_aiger_body(g, out),
        AigerMode::Binary => write_binary_aiger_body(g, out),
    }?;
    write_symbol_table(st, out)?;
    if !comments.is_empty() {
        writeln!(out, "c")?; // comment header
        writeln!(out, "{comments}")?;
    }
    Ok(())
}

fn write_ascii_aiger_body(g: &AigGraph, out: &mut impl Write) -> Result<()> {
    for &input in g.inputs() {
        debug_assert!(!input.is_inverted());
        writeln!(out, "{}", to_lit(input))?;
    }
    for &latch in g.latches() {
        debug_assert!(!latch.is_inverted());
        let next = g[latch].get_latch_input().unwrap();
        writeln!(out, "{} {}", to_lit(latch), to_lit(next))?;
    }
    for (_, _, node) in g.labels() {
        writeln!(out, "{}", to_lit(node))?;
    }

    for node in g.and_gates() {
        debug_assert!(g[node].is_and());
        let left = g[node].left();
        let right = g[node].right();
        writeln!(out, "{} {} {}", to_lit(node), to_lit(left), to_lit(right))?;
    }

    Ok(())
}
fn write_binary_aiger_body(g: &AigGraph, out: &mut impl Write) -> Result<()> {
    // For now, we check that the graph abides by all constraints and fail if it does not.
    for (idx, &input) in g.inputs().iter().enumerate() {
        assert_eq!(to_lit(input), 2 * (idx + 1));
    }
    let offset = g.inputs().len();
    for (idx, &latch) in g.latches().iter().enumerate() {
        assert_eq!(to_lit(latch), 2 * (idx + offset + 1));
    }

    for &latch in g.latches() {
        writeln!(out, "{}", to_lit(g[latch].get_latch_input().unwrap()))?;
    }
    for (_, _, source) in g.labels() {
        writeln!(out, "{}", to_lit(source))?;
    }
    let num_inputs = g.inputs().len();
    let num_latches = g.latches().len();
    for (idx, gate) in g.and_gates().enumerate() {
        let lhs_lit = to_lit(gate);
        assert_eq!(lhs_lit, 2 * (1 + num_inputs + num_latches + idx));
        let left_lit = to_lit(g[gate].left());
        let right_lit = to_lit(g[gate].right());
        let rhs0_lit = std::cmp::max(left_lit, right_lit);
        let rhs1_lit = std::cmp::min(left_lit, right_lit);
        assert!(lhs_lit >= rhs0_lit);
        let delta0 = lhs_lit - rhs0_lit;
        let delta1 = rhs0_lit - rhs1_lit;
        write_delta(out, delta0 as u64)?;
        write_delta(out, delta1 as u64)?;
    }

    Ok(())
}

fn write_delta(out: &mut impl Write, mut delta: u64) -> Result<()> {
    let mut buf = [0u8; 12];
    let mut idx = 0usize;
    while delta > 0x7f {
        let byte = (delta as u8 & 0x7f) | (1 << 7);
        buf[idx] = byte;
        idx += 1;
        delta >>= 7;
    }
    debug_assert!(delta <= 0x7f);
    buf[idx] = delta as u8;
    idx += 1;
    out.write(&buf[0..idx])?;
    Ok(())
}

fn to_lit(id: NodeId) -> usize {
    if id.is_false() {
        0
    } else if id.is_true() {
        1
    } else {
        ((usize::try_from(id).unwrap() + 1) << 1) + id.is_inverted() as usize
    }
}

fn write_header(g: &AigGraph, mode: AigerMode, out: &mut impl Write) -> Result<()> {
    let tag = mode.ext();
    let i = g.inputs().len();
    let l = g.latches().len();
    let o = g.outputs().len();
    let a = g.num_and_gates();
    let m = i + l + a;
    let b = g.bad_states().len();
    let c = g.invariants().len();
    let j = g.justice().len();
    let f = g.fairness().len();
    if b + c + j + f > 0 {
        writeln!(out, "{tag} {m} {i} {l} {o} {a} {b} {c} {j} {f}")?;
    } else {
        writeln!(out, "{tag} {m} {i} {l} {o} {a}")?;
    }
    Ok(())
}

fn write_symbol_table(st: &SymbolTable, out: &mut impl Write) -> Result<()> {
    for (kind, idx, name) in st.symbols() {
        writeln!(out, "{}{idx} {name}", kind.tag() as char)?;
    }

    Ok(())
}
