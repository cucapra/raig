//! AIGER parsing support.
//!
//! Use [`run_parser_with_options`] to parse ASCII `.aag` or binary `.aig`
//! input from any [`BufRead`] source into an [`AigGraph`].
//!
//! ```
//! use raig::aiger::run_parser_with_options;
//! use raig::graph::Value;
//! use std::io::BufReader;
//!
//! let aiger = b"aag 1 1 0 1 0\n2\n2\n";
//! let mut reader = BufReader::new(&aiger[..]);
//! let graph = run_parser_with_options(&mut reader, true)?;
//!
//! let inputs = vec![vec![Value::MAX]];
//! let trace = graph.simulate(inputs.as_slice());
//! assert_eq!(trace[0].outputs[0], Value::MAX);
//! # Ok::<(), std::io::Error>(())
//! ```

use std::io::{self, BufRead, Error};

mod ascii_parser;
mod binary_parser;
mod serializer;

use crate::graph::{AigGraph, NodeId};
use ascii_parser::parse_ascii_aiger_into_graph;
use binary_parser::parse_binary_aiger_into_graph;

pub use serializer::*;

/// Parsed metadata from an AIGER header line.
#[derive(Debug)]
pub struct AigerHeader {
    /// `true` for ASCII AIGER (`aag`), `false` for binary AIGER (`aig`).
    pub is_ascii: bool,

    /// Maximum variable index (`M` in the AIGER header).
    pub max_var: usize,

    /// Number of primary inputs.
    pub num_inputs: usize,

    /// Number of latches.
    pub num_latches: usize,

    /// Number of primary outputs.
    pub num_outputs: usize,

    /// Number of AND gates.
    pub num_and_gates: usize,

    /// Number of bad-state properties from the AIGER 1.9 extension.
    pub num_bad_states: usize,

    /// Number of invariant constraints from the AIGER 1.9 extension.
    pub num_invariants: usize,

    /// Number of justice properties from the AIGER 1.9 extension.
    pub num_justice: usize,

    /// Number of fairness constraints from the AIGER 1.9 extension.
    pub num_fairness: usize,
}

/// Parse an AIGER stream into an [`AigGraph`].
///
/// The parser accepts ASCII AIGER (`aag`) and binary AIGER (`aig`) input. When
/// `pre_optimize` is `true`, common Boolean identities are simplified while the
/// graph is constructed.
pub fn run_parser_with_options(
    reader: &mut impl BufRead,
    pre_optimize: bool,
) -> io::Result<AigGraph> {
    let header: AigerHeader = verify_aiger_header(reader)?;

    let graph: AigGraph = if header.is_ascii {
        parse_ascii_aiger_into_graph(header, reader, pre_optimize)?
    } else {
        parse_binary_aiger_into_graph(header, reader, pre_optimize)?
    };

    Ok(graph)
}

/// Parse an optional AIGER header field, which defaults to zero.
fn parse_optional_field(parser: &mut LineParser) -> usize {
    let val = parser.parse_int().unwrap_or(0);
    parser.skip_whitespace();
    val
}

/// Read and validate an AIGER header.
///
/// This consumes only the header line. It panics if the header is malformed or
/// violates the basic AIGER size constraints.
pub fn verify_aiger_header(reader: &mut impl BufRead) -> Result<AigerHeader, Error> {
    let mut parser = LineParser::default();
    parser.read_line(reader)?;
    let tag = parser.parse_word();
    let is_ascii = match tag {
        b"aag" => true,
        b"aig" => false,
        _ => panic!("Invalid tag, must be either 'aag' or 'aig'"),
    };

    // The basic header fields.
    let [max_var, num_inputs, num_latches, num_outputs, num_and_gates] = parser
        .parse_ints()
        .expect("Header must have format: aag/aig M I L O A [B C J F]");
    parser.skip_whitespace();

    // The extension header fields. The AIGER 1.9 spec says that all these
    // fields are optional; omitting them is equivalent to setting them to 0.
    let num_bad_states = parse_optional_field(&mut parser);
    let num_invariants = parse_optional_field(&mut parser);
    let num_justice = parse_optional_field(&mut parser);
    let num_fairness = parse_optional_field(&mut parser);

    assert!(parser.rest().is_empty(), "extra data on header line");

    let expected_max_var: usize = num_inputs + num_latches + num_and_gates;

    if max_var < expected_max_var {
        panic!(
            "ASCII AIGER requires M >= I + L + A, Binary requires M = I + L + A, got M={} and I+L+A={}",
            max_var, expected_max_var
        )
    }

    if max_var != expected_max_var && !is_ascii {
        panic!(
            "Binary AIGER requires M = I + L + A, got M={} and I+L+A={}",
            max_var, expected_max_var
        );
    }

    Ok(AigerHeader {
        is_ascii,
        max_var,
        num_inputs,
        num_latches,
        num_outputs,
        num_and_gates,
        num_bad_states,
        num_invariants,
        num_justice,
        num_fairness,
    })
}

/// A mapping from AIGER literal indices to our internal `NodeId`s.
#[derive(Default)]
struct Literals(Vec<NodeId>);

impl Literals {
    fn new(max_var: usize) -> Self {
        // We store one entry per *variable*: i.e., one entry in this mapping
        // covers both the regular and inverted literals corresponding to the
        // same node.
        let mut map = vec![NodeId::NONE; max_var + 1];
        map[0] = NodeId::FALSE;
        Self(map)
    }

    /// Split a literal into a variable index and an inverted flag.
    fn split(literal: usize) -> (usize, bool) {
        (literal >> 1, literal & 1 == 1)
    }

    /// Record that a given AIGER literal corresponds to a given fresh `NodeID`.
    fn add(&mut self, literal: usize, id: NodeId) {
        let (var_idx, inverted) = Self::split(literal);
        self.0[var_idx] = if inverted { id.invert() } else { id };
    }

    /// Get the `NodeID` corresponding to a given AIGER literal.
    ///
    /// Panic if the literal is not present.
    fn get(&self, literal: usize) -> NodeId {
        let (var_idx, inverted) = Self::split(literal);
        match self.0[var_idx] {
            NodeId::NONE => panic!("Unknown aiger literal: {}", literal),
            regular_node => {
                if inverted {
                    regular_node.invert()
                } else {
                    regular_node
                }
            }
        }
    }
}

/// A low-level parser for integers and words in a single ASCII line.
#[derive(Default)]
pub struct LineParser {
    /// The line buffer being parsed.
    pub buf: Vec<u8>,

    /// The current byte position in `buf`.
    pub pos: usize,
}

impl LineParser {
    /// Create a parser from an existing line buffer.
    pub fn new(buf: Vec<u8>) -> Self {
        Self { buf, pos: 0 }
    }

    /// Empty the line buffer.
    pub fn clear(&mut self) {
        self.buf.clear();
        self.pos = 0;
    }

    /// Get all the data remaining in the line buffer.
    pub fn rest(&self) -> &[u8] {
        &self.buf[self.pos..]
    }

    /// Read a line from a text-file stream. Return a flag indicating EOF.
    pub fn read_line<R: BufRead>(&mut self, reader: &mut R) -> std::io::Result<bool> {
        self.clear();
        reader.read_until(b'\n', &mut self.buf)?;
        Ok(self.buf.is_empty())
    }

    fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    fn skip(&mut self) {
        debug_assert!(self.pos < self.buf.len());
        self.pos += 1;
    }

    fn pop_if(&mut self, pred: impl Fn(u8) -> bool) -> Option<u8> {
        if let Some(byte) = self.peek()
            && pred(byte)
        {
            self.skip();
            Some(byte)
        } else {
            None
        }
    }

    /// Consume a single ASCII integer from the current point in the line buffer.
    pub fn parse_int(&mut self) -> Option<usize> {
        let mut out: Option<usize> = None;

        while let Some(byte) = self.pop_if(|b| b.is_ascii_digit()) {
            let value = byte - b'0';
            out = Some(match out {
                Some(old) => old * 10 + (value as usize),
                None => value as usize,
            });
        }

        out
    }

    /// Advance the line buffer to consume a sequence of ASCII whitespace.
    pub fn skip_whitespace(&mut self) {
        while self.pop_if(|b| b.is_ascii_whitespace()).is_some() {}
    }

    /// Consume a sequence of whitespace-separated integers.
    pub fn parse_ints<const N: usize>(&mut self) -> Option<[usize; N]> {
        // Sadly, `std::array::try_from_fn` is unstable. We fake it by setting a
        // flag on parsing errors and, on failure, wastefully continuing to
        // construct an array that we will eventually throw away.
        let mut failed = false;
        let arr = std::array::from_fn(|_| {
            self.skip_whitespace();
            match self.parse_int() {
                Some(i) => i,
                None => {
                    failed = true;
                    0
                }
            }
        });
        if failed { None } else { Some(arr) }
    }

    /// Consume a sequence of non-whitespace bytes.
    pub fn parse_word(&mut self) -> &[u8] {
        let start_pos = self.pos;
        while self.pop_if(|b| !b.is_ascii_whitespace()).is_some() {}
        &self.buf[start_pos..self.pos]
    }
}

/// A utility for reading integers from lines in a text stream.
///
/// This wraps `LineReader` in utilities for consuming lines directly from a
/// `BufRead` stream.
struct LineReader<'a, R: BufRead> {
    reader: &'a mut R,
    parser: LineParser,
}

impl<'a, R: BufRead> LineReader<'a, R> {
    fn new(reader: &'a mut R) -> Self {
        LineReader {
            reader,
            parser: LineParser::default(),
        }
    }

    /// Read a line containing a single integer.
    fn read_int(&mut self) -> std::io::Result<Option<usize>> {
        self.parser.read_line(self.reader)?;
        Ok(self.parser.parse_int())
    }

    /// Read a line containing a whitespace-separated list of integers.
    fn read_ints<const N: usize>(&mut self) -> std::io::Result<Option<[usize; N]>> {
        self.parser.read_line(self.reader)?;
        Ok(self.parser.parse_ints())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn just_int() {
        let mut parser = LineParser::new("42".as_bytes().to_vec());
        assert_eq!(parser.parse_int(), Some(42));
        assert!(parser.rest().is_empty());
    }

    #[test]
    fn int_with_stuff() {
        let mut parser = LineParser::new("42x".as_bytes().to_vec());
        assert_eq!(parser.parse_int(), Some(42));
        assert_eq!(parser.rest().len(), 1);
    }

    #[test]
    fn two_ints() {
        let mut parser = LineParser::new("42 27 x".as_bytes().to_vec());
        assert_eq!(parser.parse_ints(), Some([42, 27]));
        assert_eq!(parser.rest().len(), 2);
    }
}
