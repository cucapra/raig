# Benchmark results

Measured on October 24, 2026, at commit
[`d483908`](https://github.com/cucapra/raig/commit/d48390844bb845823f8de17916548d2e2c85f74e).
The [workflow run](https://github.com/cucapra/raig/actions/runs/40132671396)
completed successfully. Its `benchmark-results` artifact contains the raw reports.

The run used a release build on GitHub's Ubuntu 24.04 x86-64 runner, with
Rust 1.99.1, Hyperfine 1.18.0, and Valgrind 3.22.0. All 17 circuits were measured.
No other library or previous version was used as a baseline.

## Runtime

Each input was run 30 times after three warmup runs, with Hyperfine's shell
disabled. All 2,040 timed executions exited successfully. Times include process
startup, file reads, parsing, and cleanup. ASCII conversion happened before timing.

Mean command times ranged from **0.595 to 0.823 ms**. The median of the 17
per-circuit means was 0.635 ms for binary input without optimization and 0.636 ms
with optimization. For ASCII input, those medians were 0.651 and 0.667 ms.

These are small inputs: the original binary files range from 365 to 27,168 bytes.
The narrow timing range suggests that fixed command costs are substantial; this
run does not isolate parser speed. Hyperfine reported outliers, and per-case
standard deviations ranged from 0.015 to 0.063 ms. Small timing differences should
not be treated as meaningful performance differences from this single run.

## Memory

Massif profiled binary inputs with optimization disabled. The table reports the
largest recorded `mem_heap_B` value, divided by 1,024. This is requested heap
memory for the whole process, including CLI state, metadata, and temporary parser
allocations. It excludes allocator overhead and stack storage, and is neither
process RSS nor the retained graph size. Massif snapshots can miss the exact peak.

Recorded peaks ranged from **26.9 to 77.4 KiB**. `packet_switch` had the largest
peak, followed by `data_types` at 69.0 KiB and `nba_semantics` at 63.1 KiB. Several
smaller circuits clustered around 27 KiB, so these measurements do not support a
single bytes-per-node estimate.

## Per-circuit results

Runtime columns show the mean in milliseconds. “Opt” enables `--pre-optimize`.
The heap column covers binary input without optimization only.

| Circuit | Binary | Binary opt | ASCII | ASCII opt | Peak heap (KiB) |
| --- | ---: | ---: | ---: | ---: | ---: |
| case_statements | 0.615 | 0.627 | 0.646 | 0.639 | 27.0 |
| combinational_loops | 0.618 | 0.621 | 0.651 | 0.648 | 36.3 |
| counter | 0.635 | 0.595 | 0.606 | 0.632 | 26.9 |
| counter_free | 0.614 | 0.619 | 0.616 | 0.625 | 27.0 |
| counter_ones | 0.601 | 0.604 | 0.596 | 0.610 | 27.0 |
| data_types | 0.659 | 0.654 | 0.691 | 0.691 | 69.0 |
| fifo_stage | 0.634 | 0.680 | 0.695 | 0.735 | 51.2 |
| fifo_stage_bypass | 0.638 | 0.692 | 0.674 | 0.727 | 50.5 |
| fifo_stage_fail_assert | 0.648 | 0.664 | 0.667 | 0.704 | 51.3 |
| fifo_stage_fail_free | 0.650 | 0.690 | 0.683 | 0.707 | 51.4 |
| multidim_arrays | 0.616 | 0.636 | 0.637 | 0.667 | 27.0 |
| nba_semantics | 0.675 | 0.707 | 0.731 | 0.769 | 63.1 |
| package_properties | 0.642 | 0.635 | 0.646 | 0.645 | 27.0 |
| packet_switch | 0.689 | 0.759 | 0.763 | 0.823 | 77.4 |
| public_submodules | 0.606 | 0.636 | 0.619 | 0.628 | 27.0 |
| signed_operations | 0.646 | 0.687 | 0.685 | 0.729 | 34.8 |
| wire_ports | 0.619 | 0.623 | 0.621 | 0.627 | 26.9 |
