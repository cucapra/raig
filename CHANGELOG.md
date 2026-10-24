# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-10-24

### Fixed
- Stop parsing symbols at the standalone `c` separator in ASCII and binary AIGER files, preserving the comment text without panicking.
- Continue parsing constraint symbols such as `c0 constraint_name` normally.

git commit: 66c772149a12d422a5283dadec631f2675c7ab9f

## [0.2.0] - 2026-10-24

### Added
- ASCII and binary AIGER serialization, symbol tables, and comment preservation (#26).
- Internal fanin analysis for finding gates and latches that use a signal (#27).

### Changed
- `run_parser_with_options` now returns `(AigGraph, SymbolTable, String)`.
- `AigerHeader::is_ascii` is replaced by `AigerHeader::mode`, using `AigerMode`.
- Release `raig-cli` 0.1.1 with serialization support and a dependency on `raig` 0.2.0.

git commit: 439ec1de77e33a25ae619212ee79b02bc9fc782d

## [0.1.1] - 2026-08-19

### Added
- release raig-cli as a seperate crate
- hyperlink in readme to raig-cli and changelog

git commit: 5285acd900b45904c0716844d2725e65df55c467




