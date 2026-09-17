use clap::{Parser, Subcommand};
use raig::aiger::{AigerMode, run_parser_with_options, write_aiger_with_symbol_table_and_comments};
use raig::graph;
use raig::graph::SymbolTable;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about = "AIGER command-line tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Parse an AIGER file, but do not write any .dot output
    Parse {
        /// Input .aag/.aig file, or '-' to read from stdin
        input: String,

        /// Optimize while constructing the graph
        #[arg(long)]
        pre_optimize: bool,

        /// Print AIG graph
        #[arg(long)]
        print: bool,
    },

    /// Simulate an AIGER circuit using a stimulus file
    Simulate {
        /// Input .aag/.aig file
        input: String,

        /// Stimulus file containing one 0/1 input vector per line
        stimulus: PathBuf,

        /// Optimize while constructing the graph
        #[arg(long)]
        pre_optimize: bool,

        /// Print a labeled, human-readable trace
        #[arg(long)]
        pretty: bool,
    },

    /// Convert an ASCII AIGER file to binary AIGER, or binary AIGER to ASCII
    Convert {
        /// Input .aag/.aig file, or '-' to read from stdin
        input: String,

        /// Output .aag/.aig name and location file
        /// examples:
        ///   --output aiger.aag
        ///   --output ./aiger.aag
        ///   --output /Users/Modi/Projects/AIG/aiger.aag
        #[arg(short, long, value_parser = parse_aiger_output_path)]
        output: Option<PathBuf>,
    },

    /// Parse an AIGER file and produce Graphviz DOT output
    Dot {
        /// Input .aag/.aig file, or '-' to read from stdin
        input: String,

        /// Optimize while constructing the graph
        #[arg(long)]
        pre_optimize: bool,

        /// Output .dot name and location file
        /// examples:
        ///   --output graph.dot
        ///   --output ./graph.dot
        ///   --output /Users/Modi/Projects/AIG/graph.dot
        #[arg(short, long, value_parser = parse_dot_path)]
        output: Option<PathBuf>,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse {
            input,
            pre_optimize,
            print,
        } => {
            let graph = parse_input(&input, pre_optimize)?;

            if print {
                println!("{graph:#?}");
            }
        }

        Commands::Simulate {
            input,
            stimulus,
            pre_optimize,
            pretty,
        } => {
            let (graph, _, _) = parse_input(&input, pre_optimize)?;

            let stimulus_file = File::open(&stimulus)?;
            let stimulus_reader = BufReader::new(stimulus_file);
            let stimulus_parser = graph::StimulusParser::new(stimulus_reader);

            let trace = graph.simulate(stimulus_parser);

            if pretty {
                print_pretty_trace(&trace);
            } else {
                print_aiger_trace(&trace);
            }
        }

        Commands::Convert { input, output } => {
            let (graph, st, comments) = parse_input(&input, false)?;
            if let Some(out) = output {
                let mut out_writer = BufWriter::new(File::create(&out)?);
                match out.extension().and_then(OsStr::to_str) {
                    Some("aag") => write_aiger_with_symbol_table_and_comments(
                        &graph,
                        &st,
                        &comments,
                        AigerMode::Ascii,
                        &mut out_writer,
                    )?,
                    Some("aig") => write_aiger_with_symbol_table_and_comments(
                        &graph,
                        &st,
                        &comments,
                        AigerMode::Binary,
                        &mut out_writer,
                    )?,
                    _ => eprintln!("Unsupported file extension"),
                }
            } else {
                write_aiger_with_symbol_table_and_comments(
                    &graph,
                    &st,
                    &comments,
                    AigerMode::Ascii,
                    &mut io::stdout(),
                )?;
            }
        }

        Commands::Dot {
            input,
            pre_optimize,
            output,
        } => {
            let (graph, _, _) = parse_input(&input, pre_optimize)?;
            let dot: String = graph.to_dot();

            if let Some(output) = output {
                fs::write(&output, &dot)?;
                println!("Wrote dot file to {}", output.display());
            } else {
                print!("{}", dot);
            }
        }
    }

    Ok(())
}

fn parse_input(
    input: &str,
    pre_optimize: bool,
) -> io::Result<(graph::AigGraph, SymbolTable, String)> {
    if input == "-" {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin.lock());

        run_parser_with_options(&mut reader, pre_optimize)
    } else {
        let file = File::open(input)?;
        let mut reader = BufReader::new(file);

        run_parser_with_options(&mut reader, pre_optimize)
    }
}

fn parse_dot_path(s: &str) -> Result<PathBuf, String> {
    if s.ends_with(".dot") {
        Ok(PathBuf::from(s))
    } else {
        Err(format!("output file must end with .dot: {s}"))
    }
}

fn parse_aiger_output_path(s: &str) -> Result<PathBuf, String> {
    if s.ends_with(".aag") || s.ends_with(".aig") {
        Ok(PathBuf::from(s))
    } else {
        Err(format!("output file must end with .aag or .aig: {s}"))
    }
}

/// Convert simulation values into a string such as "0101".
fn values_to_bits(values: &[graph::Value]) -> String {
    values
        .iter()
        .map(|&value| if value == 0 { '0' } else { '1' })
        .collect()
}

/// Print the transition format used by the C AIGER simulator:
///
/// current-state inputs outputs current-state
fn print_aiger_trace(trace: &[graph::SimulationStep]) {
    for step in trace {
        println!(
            "{} {} {} {}",
            values_to_bits(&step.state),
            values_to_bits(&step.inputs),
            values_to_bits(&step.outputs),
            values_to_bits(&step.state),
        );
    }
}

/// Print a labeled, human-readable simulation trace.
fn print_pretty_trace(trace: &[graph::SimulationStep]) {
    for (time_step, step) in trace.iter().enumerate() {
        println!("Time step {time_step}:");
        println!("     Current state: {}", pretty_values(&step.state));
        println!("     Inputs: {}", pretty_values(&step.inputs));
        println!("     Outputs: {}", pretty_values(&step.outputs));
        println!("     Next state: {}", pretty_values(&step.next_state));

        if time_step + 1 < trace.len() {
            println!();
        }
    }
}

/// Display "-" when a circuit has no values in a category, such as a
/// combinational circuit with no latches.
fn pretty_values(values: &[graph::Value]) -> String {
    if values.is_empty() {
        "-".to_string()
    } else {
        values_to_bits(values)
    }
}
