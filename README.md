# raig

`raig` (pronounced “rage”) is a dependency-free library for working with
[AIGs](https://en.wikipedia.org/wiki/And-inverter_graph)
in Rust, developed at
[Cornell's Capra Lab](https://capra.cs.cornell.edu/).

An AIG represents Boolean logic with AND gates and inverted edges. This compact
form is useful for logic verification, synthesis, and testing tools.

The command-line tool is published as the
separate [`raig-cli`](https://crates.io/crates/raig-cli) crate, which depends on `raig` and `clap`.

For relevant version updates, see the [`changelog`](https://github.com/cucapra/raig/blob/main/CHANGELOG.md) file.

## Installation

Add the library to a Rust project:

```sh
cargo add raig
```

Install the command-line tool:

```sh
cargo install raig-cli
```

## License

Licensed under the MIT license.
