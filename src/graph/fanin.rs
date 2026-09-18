// Copyright 2026 Cornell University
// released under MIT License
// author: Kevin Laeufer <laeufer@cornell.edu>

use crate::graph::{AigGraph, NodeId};

/// Provides access to all the nodes a signal fans into.
pub struct FanIn {
    ends: Vec<u32>,
    list: Vec<NodeId>,
}

fn count_uses(g: &AigGraph) -> Vec<u32> {
    let mut counts = vec![0; g.nodes.len() + 2];
    for node in &g.nodes {
        for child in [node.left(), node.right()] {
            if let Some(idx) = get_index(child) {
                counts[idx] += 1;
            }
        }
    }
    counts
}

/// For fan in analysis we do assign true and false their own slots and thus there is an offset of 2
#[inline]
fn get_index(node: NodeId) -> Option<usize> {
    if node.is_true() {
        Some(1)
    } else if node.is_false() {
        Some(0)
    } else if let Ok(ii) = usize::try_from(node) {
        Some(ii + 2)
    } else {
        None
    }
}

impl FanIn {
    pub fn from_graph(g: &AigGraph) -> Self {
        // We first need to count how many latches and `and` gates use the particular node.
        // This allos us to appropriately allocate space.
        let use_counts = count_uses(g);
        let total_uses: usize = use_counts.iter().map(|c| *c as usize).sum();

        let mut list = vec![NodeId::NONE; total_uses];
        let mut ends = Vec::with_capacity(use_counts.len());
        let mut prev = 0;
        for count in use_counts {
            let next = prev + count;
            ends.push(next);
            prev = next;
        }

        // fill list
        let mut offset = vec![0u32; ends.len()];
        for (node_idx, node) in g.nodes.iter().enumerate() {
            let node_id: NodeId = node_idx.into();
            for child in [node.left(), node.right()] {
                if let Some(idx) = get_index(child) {
                    let start = if idx == 0 { 0 } else { ends[idx - 1] };
                    let list_idx = (start + offset[idx]) as usize;
                    debug_assert_eq!(list[list_idx], NodeId::NONE);
                    list[list_idx] = node_id;
                    offset[idx] += 1;
                }
            }
        }

        Self { ends, list }
    }

    pub fn nodes(&self, source: NodeId) -> &[NodeId] {
        let (start, end) = self.get_range(source).unwrap();
        self.list[start as usize..end as usize].as_ref()
    }

    pub fn count(&self, source: NodeId) -> u32 {
        if let Some((start, end)) = self.get_range(source) {
            end - start
        } else {
            0
        }
    }

    fn get_range(&self, node: NodeId) -> Option<(u32, u32)> {
        if let Some(index) = get_index(node) {
            let start = if index == 0 { 0 } else { self.ends[index - 1] };
            let end = self.ends[index];
            Some((start, end))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aiger::run_parser_with_options;

    fn validate_fan_in(g: &AigGraph, f: &FanIn) {
        for index in 0..g.nodes.len() {
            let source: NodeId = index.into();
            for &target in f.nodes(source) {
                let children = [g[target].left().regular(), g[target].right().regular()];
                assert!(children.contains(&source.regular()));
            }
        }
    }

    fn load_graph(filename: &str) -> AigGraph {
        let f = std::fs::File::open(filename).unwrap();
        let mut r = std::io::BufReader::new(f);
        let (g, _, _) = run_parser_with_options(&mut r, true).unwrap();
        g
    }

    #[test]
    fn test_fan_in() {
        let filename = "tests/inputs/hrt/fifo_stage_bypass.aig";
        let g = load_graph(filename);
        let f = FanIn::from_graph(&g);
        validate_fan_in(&g, &f);
    }
}
