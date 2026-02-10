# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                          # dev build
cargo build --release                # release build (LTO, strip, panic=abort)
cargo test --verbose --all           # run all workspace tests
cargo test --package gcrecomp-core   # tests for a specific crate
cargo test test_name                 # run a single test by name
cargo test -- --nocapture            # run tests with stdout visible
cargo fmt --all -- --check           # check formatting (CI enforced)
cargo clippy --all-targets --all-features -- -D warnings  # lint (CI enforced)
```

Debug logging: `RUST_LOG=debug cargo run --bin gcrecomp-cli -- [args]`
Module-level logging: `RUST_LOG=gcrecomp_core::recompiler=debug cargo run`

## Project Overview

GCRecomp is a static recompiler that translates GameCube PowerPC DOL binaries into Rust code, producing native PC executables. Inspired by [N64Recomp](https://github.com/N64Recomp/N64Recomp).

## Workspace Crates

| Crate | Purpose |
|---|---|
| `gcrecomp-core` | Core recompiler: parser, decoder, analysis, codegen, pipeline |
| `gcrecomp-cli` | CLI binary (`gcrecomp-cli`) |
| `gcrecomp-runtime` | Runtime for recompiled games: memory (RAM/VRAM/ARAM), graphics (wgpu), input |
| `gcrecomp-ui` | GUI (Iced-based) |
| `gcrecomp-lua` | Lua scripting via `mlua` (Lua 5.4, vendored) |
| `gcrecomp-web` | Web dashboard (Axum + Tokio) |
| `game` | Output crate containing auto-generated recompiled Rust code |

## Recompilation Pipeline (gcrecomp-core)

The core pipeline lives in `gcrecomp-core/src/recompiler/` and flows:

1. **Parser** (`parser.rs`) — parses DOL file headers and sections
2. **Ghidra Analysis** (`ghidra/`) — imports function metadata, symbols, types from Ghidra export (Python script: `scripts/ghidra_export.py`)
3. **Decoder** (`decoder.rs`) — decodes 32-bit PowerPC instructions into typed `Instruction` structs
4. **Analysis** (`analysis/`) — control flow graph, data flow (def-use chains, live variables), type inference
5. **IR** (`ir/`) — intermediate representation with optimization passes
6. **Codegen** (`codegen/`) — generates Rust source from IR; includes function dispatcher
7. **Validator** — syntax/semantic checks on generated code
8. **Pipeline** (`pipeline.rs`) — orchestrates all stages; also customizable via Lua scripts (`lua/pipeline.lua`)

## Key Conventions

- Rust 2021 edition, MSRV 1.70
- Formatting: `rustfmt` with max_width=100, 4-space indentation, Unix newlines (see `.rustfmt.toml`)
- Linting: clippy with `-D warnings` in CI; config in `.clippy.toml`
- Error handling: `thiserror` for typed errors, `anyhow` for CLI/top-level
- Memory optimization patterns used throughout: `#[repr(u8)]` enums, `SmallVec` for small collections, `BitVec` for register/block sets, `#[repr(packed)]` structs
- Shared dependencies are declared at workspace level in root `Cargo.toml` under `[workspace.dependencies]`

## CI

GitHub Actions (`.github/workflows/ci.yml`) runs on push/PR to `main`/`develop`:
- Format check, clippy, build, and tests across Ubuntu/macOS/Windows with stable/beta (nightly on Ubuntu only)
- Security audit via `cargo audit`
- Release builds uploaded as artifacts
