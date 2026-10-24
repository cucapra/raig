//! Graph data structures and simulation utilities for AIGs.
//!
//! Most users start with [`AigBuilder`], add inputs, latches, AND gates, and
//! outputs, then call [`AigBuilder::build`] to get an [`AigGraph`].

use std::collections::HashMap;
use std::ops::Index;

mod eval;
mod fanin;
mod graphviz;
mod stimulus;
mod symbols;

pub use eval::{SimulationStep, Simulator, Value};
pub use stimulus::{Stimulus, StimulusParser};
pub use symbols::{SymbolKind, SymbolTable};

/// An identifier for a signal in an AIG.
///
/// A `NodeId` can refer to a constant, an input, a latch, an AND node, or an
/// inverted version of one of those signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(u32);

const INVERSION_MASK: u32 = 0b0000_0000_0000_0000_0000_0000_0000_0001;
const NODE_ID_MASK: u32 = 0b1111_1111_1111_1111_1111_1111_1111_1110;

/// One node in an AIG graph.
///
/// Inputs and latches are represented as marker nodes. AND nodes store their
/// left and right input signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AigNode {
    left: NodeId,
    right: NodeId,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum LabelKind {
    Output,
    BadState,
    Constraints,
    Justice,
    Fairness,
}

/// A built AIG that can be evaluated, simulated, or rendered as DOT.
#[derive(Debug)]
pub struct AigGraph {
    nodes: Vec<AigNode>,
    inputs: Vec<NodeId>,
    latches: Vec<NodeId>,
    outputs: Vec<NodeId>,
    bad_states: Vec<NodeId>,
    constraints: Vec<NodeId>,
    justice: Vec<NodeId>,
    fairness: Vec<NodeId>,
}

/// Incrementally builds an [`AigGraph`].
///
/// Use [`AigBuilder::add_and_optimized`] to canonicalize and simplify common
/// AND patterns while constructing the graph.
#[derive(Debug)]
pub struct AigBuilder {
    graph: AigGraph,
    and_hash: HashMap<AigNode, NodeId>,
}

impl NodeId {
    /// The false constant.
    pub const FALSE: NodeId = NodeId(0);

    /// The true constant.
    pub const TRUE: NodeId = NodeId(1);

    /// Reserved marker used inside [`AigNode`] to represent an input or latch.
    /// This should never be a real graph node ID.
    pub const NONE: NodeId = NodeId(NODE_ID_MASK);

    /// Return whether this signal is inverted.
    pub fn is_inverted(self) -> bool {
        (self.0 & INVERSION_MASK) != 0
    }

    /// Return this signal without its inversion bit.
    pub fn regular(self) -> Self {
        Self(self.0 & NODE_ID_MASK)
    }

    /// Return the inverted form of this signal.
    pub fn invert(self) -> Self {
        Self(self.0 ^ INVERSION_MASK)
    }

    /// Return whether this signal is one of the two constants.
    pub fn is_const(self) -> bool {
        self.regular() == NodeId::FALSE
    }

    /// Marker values are reserved for classifying input and latch nodes.
    pub fn is_marker(self) -> bool {
        self.regular() == Self::NONE
    }

    /// Return whether this signal is the false constant.
    pub fn is_false(self) -> bool {
        self == NodeId::FALSE
    }

    /// Return whether this signal is the true constant.
    pub fn is_true(self) -> bool {
        self == NodeId::TRUE
    }

    fn index(self) -> usize {
        usize::try_from(self).expect("NodeId does not correspond to a graph index")
    }
}

/// Convert a non-constant, non-marker `NodeId` to the corresponding graph index.
///
/// Constants and marker values are not stored as ordinary graph nodes, so they
/// cannot be converted into indices.
///
/// The graph vector is zero-indexed, while real node IDs start after the
/// constants and reserve the least significant bit for inversion:
///
/// ```text
/// graph[0] -> NodeId(2)
/// graph[1] -> NodeId(4)
/// graph[2] -> NodeId(6)
/// NodeId(7) = inverted NodeId(6)
/// ```
impl TryFrom<NodeId> for usize {
    type Error = &'static str;

    fn try_from(id: NodeId) -> Result<Self, Self::Error> {
        let regular_id = id.regular();

        if regular_id.is_const() {
            return Err("constants are not stored in the graph");
        }

        if regular_id.is_marker() {
            return Err("input/latch marker is not stored in the graph");
        }

        Ok(((regular_id.0 >> 1) - 1) as usize)
    }
}

/// Convert a graph index to its regular, non-inverted `NodeId`.
///
/// ```text
/// graph[0] -> NodeId(2)
/// graph[1] -> NodeId(4)
/// graph[2] -> NodeId(6)
/// ```
impl From<usize> for NodeId {
    fn from(index: usize) -> Self {
        // We reserve NODE_ID_MASK / INPUT_NODE_MARKER as a special marker,
        // so the largest real graph NodeId must be smaller than that.
        const MAX_GRAPH_INDEX: usize = (u32::MAX as usize / 2) - 2;

        assert!(
            index <= MAX_GRAPH_INDEX,
            "graph index {index} does not fit in NodeId"
        );

        Self(((index + 1) * 2) as u32)
    }
}

impl AigNode {
    fn new(left: NodeId, right: NodeId) -> Self {
        Self { left, right }
    }

    fn new_input() -> Self {
        Self {
            left: NodeId::NONE,
            right: NodeId::NONE,
        }
    }

    fn new_latch(latch_input: NodeId) -> Self {
        Self {
            left: NodeId::NONE,
            right: latch_input,
        }
    }

    /// Return the left input signal for this node.
    ///
    /// This is meaningful for AND nodes. For inputs and latches, this is an
    /// internal marker value.
    pub fn left(&self) -> NodeId {
        self.left
    }

    /// Return the right input signal for this node.
    ///
    /// For latches, this is the next-state signal.
    pub fn right(&self) -> NodeId {
        self.right
    }

    /// Return whether this node is an input marker.
    pub fn is_input(&self) -> bool {
        self.left.is_marker() && self.right.is_marker()
    }

    /// Return whether this node is a latch marker.
    pub fn is_latch(&self) -> bool {
        self.left.is_marker() && !self.right.is_marker()
    }

    /// Return whether this node is an AND gate.
    pub fn is_and(&self) -> bool {
        !self.left.is_marker()
    }

    /// Set the next-state signal for a latch.
    ///
    /// Panics if this node is not a latch.
    pub fn set_latch_input(&mut self, latch_input: NodeId) {
        assert!(
            self.is_latch(),
            "Tried to set the input of a non-latch node"
        );

        self.right = latch_input;
    }

    pub fn get_latch_input(&self) -> Option<NodeId> {
        if self.is_latch() {
            Some(self.right)
        } else {
            None
        }
    }
}

impl AigGraph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            inputs: Vec::new(),
            latches: Vec::new(),
            outputs: Vec::new(),
            bad_states: vec![],
            constraints: vec![],
            justice: vec![],
            fairness: vec![],
        }
    }

    /// Return a mutable reference to a graph node by ID.
    ///
    /// This is mainly useful for filling latch next-state signals after the
    /// latch has been created.
    pub fn node(&mut self, id: NodeId) -> &mut AigNode {
        &mut self.nodes[id.index()]
    }

    pub fn inputs(&self) -> &[NodeId] {
        &self.inputs
    }
    pub fn outputs(&self) -> &[NodeId] {
        &self.outputs
    }

    pub fn latches(&self) -> &[NodeId] {
        &self.latches
    }

    pub fn bad_states(&self) -> &[NodeId] {
        &self.bad_states
    }

    pub fn invariants(&self) -> &[NodeId] {
        &self.constraints
    }

    pub fn justice(&self) -> &[NodeId] {
        &self.justice
    }

    pub fn fairness(&self) -> &[NodeId] {
        &self.fairness
    }

    pub fn num_and_gates(&self) -> usize {
        // there are only three kinds of nodes: inputs, latches and and gates
        self.nodes.len() - self.inputs.len() - self.latches.len()
    }

    pub fn and_gates(&self) -> impl Iterator<Item = NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.is_and())
            .map(|(idx, _)| NodeId::from(idx))
    }

    /// Returns an iterator over all labels in the following order:
    /// output -> bad_states -> constraints -> justice -> fairness
    pub fn labels(&self) -> impl Iterator<Item = (LabelKind, usize, NodeId)> {
        self.outputs
            .iter()
            .enumerate()
            .map(|(i, &n)| (LabelKind::Output, i, n))
            .chain(
                self.bad_states
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| (LabelKind::BadState, i, n)),
            )
            .chain(
                self.constraints
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| (LabelKind::Constraints, i, n)),
            )
            .chain(
                self.justice
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| (LabelKind::Justice, i, n)),
            )
            .chain(
                self.fairness
                    .iter()
                    .enumerate()
                    .map(|(i, &n)| (LabelKind::Fairness, i, n)),
            )
    }
}

impl AigBuilder {
    /// Create a new empty builder.
    pub fn new() -> Self {
        Self {
            graph: AigGraph::new(),
            and_hash: HashMap::new(),
        }
    }

    /// Finish building and return the graph.
    pub fn build(self) -> AigGraph {
        self.graph
    }

    /// Return a mutable reference to a node already added to the graph.
    pub fn node(&mut self, id: NodeId) -> &mut AigNode {
        self.graph.node(id)
    }

    /// Add a primary input and return its signal ID.
    pub fn add_input(&mut self) -> NodeId {
        let index = self.graph.nodes.len();
        let id = NodeId::from(index);

        self.graph.nodes.push(AigNode::new_input());
        self.graph.inputs.push(id);

        id
    }

    /// Add a latch initialized to a next-state signal.
    ///
    /// If the next-state signal is not known yet, pass [`NodeId::FALSE`] and
    /// update the latch later with [`AigNode::set_latch_input`].
    pub fn add_latch(&mut self, latch_input: NodeId) -> NodeId {
        let index = self.graph.nodes.len();
        let id = NodeId::from(index);

        self.graph.nodes.push(AigNode::new_latch(latch_input));
        self.graph.latches.push(id);

        id
    }

    /// Add an AND gate without simplification or structural hashing.
    pub fn add_and_raw(&mut self, left: NodeId, right: NodeId) -> NodeId {
        let index = self.graph.nodes.len();
        let id = NodeId::from(index);

        self.graph.nodes.push(AigNode::new(left, right));

        id
    }

    /// Add an AND gate with simple Boolean simplification and structural hashing.
    ///
    /// This reuses equivalent AND nodes and simplifies patterns such as
    /// `x & true`, `x & false`, `x & x`, and `x & !x`.
    pub fn add_and_optimized(&mut self, left: NodeId, right: NodeId) -> NodeId {
        // x & false = false
        if left.is_false() || right.is_false() {
            return NodeId::FALSE;
        }

        // x & true = x
        if left.is_true() {
            return right;
        }

        if right.is_true() {
            return left;
        }

        // x & x = x
        if left == right {
            return left;
        }

        // x & !x = false
        if left == right.invert() {
            return NodeId::FALSE;
        }

        // AND is commutative, so canonicalize child order.
        let (left, right) = if right < left {
            (right, left)
        } else {
            (left, right)
        };

        let node = AigNode::new(left, right);

        if let Some(existing_id) = self.and_hash.get(&node) {
            return *existing_id;
        }

        let index = self.graph.nodes.len();
        let id = NodeId::from(index);

        self.graph.nodes.push(node);
        self.and_hash.insert(node, id);

        id
    }

    /// Add a primary output signal.
    pub fn add_output(&mut self, output: NodeId) {
        self.graph.outputs.push(output);
    }

    pub fn add_bad_state(&mut self, output: NodeId) {
        self.graph.bad_states.push(output);
    }

    pub fn add_invariant(&mut self, output: NodeId) {
        self.graph.constraints.push(output);
    }

    pub fn add_justice(&mut self, output: NodeId) {
        self.graph.justice.push(output);
    }

    pub fn add_fairness(&mut self, output: NodeId) {
        self.graph.fairness.push(output);
    }
}

impl Default for AigGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<NodeId> for AigGraph {
    type Output = AigNode;

    fn index(&self, id: NodeId) -> &Self::Output {
        &self.nodes[id.index()]
    }
}
