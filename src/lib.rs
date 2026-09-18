#![doc(
    html_logo_url = "https://tr.rbxcdn.com/180DAY-2de3012b73f0302860041299c84dc0ac/420/420/Hat/Webp/noFilter"
)]
//! `raig` (pronounced “rage”) is a dependency-free library for working with
//! [AIGs](https://en.wikipedia.org/wiki/And-inverter_graph)
//! in Rust, developed at
//! [Cornell's Capra Lab](https://capra.cs.cornell.edu/).
//! AIGs represent Boolean logic using only AND nodes and NOT edges. This simple
//! structure makes them smaller and faster for computer tools to process than
//! other logic representations.
//!
//! One of `raig`'s key advantages is its use of
//! compact [`graph::NodeId`] values instead of pointers.
//!
//! # Tutorial
//!
//! ## 1.0 Start With an AIGER File
//!
//! The most common starting point is an
//! [AIGER file](https://fmv.jku.at/aiger/FORMAT.aiger)
//! (a digital file format for storing AIGs)
//! from another tool, such as
//! [py-aiger](https://github.com/mvcisback/py-aiger)
//! or
//! [aiger](https://github.com/arminbiere/aiger).
//! Internal generation of AIGER files is not yet supported, but you can bypass the parser and build a
//! [`graph::AigGraph`] (our intenral representation of AIGs) directly using [`graph::AigBuilder`].
//!
//! As of 0.1.0, `raig`'s AIGER parser accepts anything that implements [`std::io::BufRead`], so the same parser
//! works for local files, stdin, network responses, and uploaded file bytes.
//!
//! ### 1.1 More about AIGER files
//!
//! Here is a small example demonstrating some of AIGER's invariants, but you can (and should!)
//! read more about the
//! invariants of AIGER files (i.e., .aag and .aig files)
//! [here](https://github.com/arminbiere/aiger).
//!
//! This is an example ASCII AIGER circuit that has one input and one output. The output is
//! exactly the input, so it behaves like an identity function.
//!
//! ```text
//! aag 1 1 0 1 0
//! 2
//! 2
//! ```
//!
//! The header is:
//!
//! ```text
//! aag M I L O A
//! ```
//!
//! In this example, `M = 1`, `I = 1`, `L = 0`, `O = 1`, and `A = 0`: one input,
//! no latches, one output, and no AND gates. The first body line declares input
//! literal `2`; the second body line says the output is also literal `2`.
//!
//! ### 1.2 Parse Uploaded Bytes
//!
//! If an application receives uploaded AIGER bytes, wrap those bytes in a
//! [`std::io::BufReader`] and call [`aiger::run_parser_with_options`].
//!
//! ```
//! use raig::aiger::run_parser_with_options;
//! use std::io::BufReader;
//!
//! let uploaded_aiger = b"aag 1 1 0 1 0\n2\n2\n";
//! let mut reader = BufReader::new(&uploaded_aiger[..]);
//! let (graph, _, _) = run_parser_with_options(&mut reader, true)?;
//!
//! # let _ = graph;
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! ### 1.3 Parse a File From Disk
//!
//! Local files use the same API. Open the file, wrap it in a buffered reader,
//! and parse it.
//!
//! ```no_run
//! use raig::aiger::run_parser_with_options;
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let file = File::open("circuit.aag")?;
//! let mut reader = BufReader::new(file);
//! let (graph, _, _) = run_parser_with_options(&mut reader, true)?;
//!
//! # let _ = graph;
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! ## 2.0 Work With the Parsed [`graph::AigGraph`]
//!
//! After parsing, you now have an internal AIG representation! Congrats! From there, the most
//! common next steps are simulation and visualization (AIGER conversions and local rewriting and upcoming features).
//!
//! ### 2.1 Simulate the Circuit
//!
//! Simulation inputs are vectors of [`graph::Value`]. Use `0` for false and
//! [`graph::Value::MAX`] for true.
//!
//! ```
//! use raig::aiger::run_parser_with_options;
//! use raig::graph::Value;
//! use std::io::BufReader;
//!
//! let uploaded_aiger = b"aag 1 1 0 1 0\n2\n2\n";
//! let mut reader = BufReader::new(&uploaded_aiger[..]);
//! let (graph, _, _) = run_parser_with_options(&mut reader, true)?;
//!
//! let stimulus = vec![
//!     vec![0],
//!     vec![Value::MAX],
//! ];
//! let trace = graph.simulate(stimulus.as_slice());
//!
//! assert_eq!(trace[0].outputs[0], 0);
//! assert_eq!(trace[1].outputs[0], Value::MAX);
//! # Ok::<(), std::io::Error>(())
//! ```
//!
//! `Value` is a packed Boolean value. Other bit patterns can be used to
//! evaluate many independent Boolean lanes in parallel with bitwise operations.
//!
//! ### 2.2 Render Graphviz DOT
//!
//! Once parsed, an [`graph::AigGraph`] can be rendered as Graphviz DOT for
//! inspection.
//!
//! ```
//! use raig::aiger::run_parser_with_options;
//! use std::io::BufReader;
//!
//! let uploaded_aiger = b"aag 1 1 0 1 0\n2\n2\n";
//! let mut reader = BufReader::new(&uploaded_aiger[..]);
//! let (graph, _, _) = run_parser_with_options(&mut reader, true)?;
//!
//! let dot = graph.to_dot();
//! assert!(dot.contains("digraph AIG"));
//! # Ok::<(), std::io::Error>(())
//! ```
//! It should be noted that this feature is only especially
//! helpful for AIG graphs on the smaller side, and is not so efficient for larger graphs.
//!
//! ## 3.0 Use the CLI
//!
//! Install the CLI with:
//!
//! ```sh
//! cargo install raig-cli
//! ```
//!
//! Then parse, render, or simulate AIGER files:
//!
//! ```sh
//! raig parse circuit.aag
//! raig dot circuit.aag --output circuit.dot
//! raig simulate circuit.aag stimulus.txt
//! ```
//!
//! Stimulus files contain one input vector per line and may end with `.`:
//!
//! ```text
//! 0
//! 1
//! .
//! ```
//!
//! ## 4.0 Build Graphs Directly
//!
//! You can also build AIGs directly in Rust with [`graph::AigBuilder`]. This is
//! useful for generating circuits, writing tests, or integrating with another
//! frontend.
//!
//! ### 4.1 Build an AND Gate
//!
//! ```
//! use raig::graph::{AigBuilder, Value};
//!
//! let mut builder = AigBuilder::new();
//! let a = builder.add_input();
//! let b = builder.add_input();
//! let output = builder.add_and_optimized(a, b);
//! builder.add_output(output);
//!
//! let graph = builder.build();
//! let inputs = vec![
//!     vec![Value::MAX, Value::MAX],
//!     vec![Value::MAX, 0],
//! ];
//! let trace = graph.simulate(inputs.as_slice());
//!
//! assert_eq!(trace[0].outputs[0], Value::MAX);
//! assert_eq!(trace[1].outputs[0], 0);
//! ```
//!
//! ### 4.2 Use Inverted Edges
//!
//! AIGs do not store NOT gates as separate edges. Instead, a [`graph::NodeId`]
//! can refer to either a regular signal or its inverted form:
//!
//! ```
//! use raig::graph::{AigBuilder, Value};
//!
//! let mut builder = AigBuilder::new();
//! let a = builder.add_input();
//! let b = builder.add_input();
//!
//! // OR via De Morgan's law:
//! // a | b = !(!a & !b)
//! let not_a_and_not_b = builder.add_and_optimized(a.invert(), b.invert());
//! builder.add_output(not_a_and_not_b.invert());
//!
//! let graph = builder.build();
//! let inputs = vec![
//!     vec![0, 0],
//!     vec![Value::MAX, 0],
//!     vec![0, Value::MAX],
//! ];
//! let trace = graph.simulate(inputs.as_slice());
//!
//! assert_eq!(trace[0].outputs[0], 0);
//! assert_eq!(trace[1].outputs[0], Value::MAX);
//! assert_eq!(trace[2].outputs[0], Value::MAX);
//! ```

pub mod aiger;
pub mod graph;
