// Copyright 2026 Cornell University
// released under MIT License
// author: Kevin Laeufer <laeufer@cornell.edu>

use crate::graph::{AigGraph, NodeId};
use std::io::{Result, Write};
pub fn write_ascii_aiger(g: &AigGraph, out: &mut impl Write) -> Result<()> {
    write_header(g, true, out)?;
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

fn to_lit(id: NodeId) -> usize {
    if id.is_false() {
        0
    } else if id.is_true() {
        1
    } else {
        ((usize::try_from(id).unwrap() + 1) << 1) + id.is_inverted() as usize
    }
}

pub fn write_binary_aiger(g: &AigGraph, out: &mut impl Write) -> Result<()> {
    write_header(g, false, out)?;

    todo!()
}

fn write_header(g: &AigGraph, is_ascii_not_binary: bool, out: &mut impl Write) -> Result<()> {
    let tag = if is_ascii_not_binary { "aag" } else { "aig" };
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
