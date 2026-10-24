# Benchmarks

Run the Benchmarks workflow from GitHub Actions, then download the
`benchmark-results` artifact. The workflow measures the current version of `raig` using a release build.

## Memory

[Massif](https://valgrind.org/docs/manual/ms-manual.html) measures heap usage while
`raig parse` loads each circuit, including temporary parser allocations.
The artifact contains the profiles and text reports.

## Runtime

[Hyperfine](https://github.com/sharkdp/hyperfine) runs `raig parse` 30 times per
circuit after three warmup runs. It measures binary and ASCII inputs, with
optimization enabled and disabled. ASCII conversion happens before timing.

The JSON reports contain elapsed times for the complete command, including
process startup, file reads, parsing, and cleanup.
