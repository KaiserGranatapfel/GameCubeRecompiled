# GCRecomp

**Experimental static recompiler for GameCube games → Rust → Native PC executables**

GCRecomp is an ambitious and exciting project that statically translates GameCube PowerPC binaries (DOL files) into Rust code, producing standalone, cross-platform executables (`game.exe` on Windows, `game` on Linux/macOS) that run natively on modern PCs without any emulator.

Inspired by the groundbreaking [N64Recomp](https://github.com/N64Recomp/N64Recomp), GCRecomp aims to preserve classic GameCube titles, enable high-performance ports, and open new possibilities for modding and analysis — all while leveraging Rust's safety, performance, and ecosystem.

This is one of the cooler and more technically challenging projects in my GitHub portfolio: combining reverse engineering, compiler design, low-level systems programming, and game preservation in a single effort.

## Current Status

- Early proof-of-concept stage
- Successfully parses DOL file headers and extracts `.text` sections
- Basic PowerPC instruction decoding and manual/hand-crafted reimplementation in Rust
- Minimal runtime with CPU context (registers, 24 MB RAM model) and memory access
- Generates and runs simple recompiled functions in standalone binaries
- Cross-platform builds verified on Windows and Linux (via WSL2)

## Planned Features & Roadmap

The project is designed to grow incrementally, with a strong focus on modularity, correctness, and extensibility.

### Short-Term Goals
- Automated instruction decoding and Rust code generation for small functions
- Support for common integer instructions (add, subi, cmpwi, branches, loads/stores)
- Stack frame handling (stwu, mflr/mtlr, etc.)
- Basic SDK stubs (OSReport, memory functions)
- Fully automated recompilation pipeline: DOL → Ghidra analysis → generated Rust → standalone binary

### Medium-Term Goals
- Recompile menu/main loop functions from real games
- Enhanced runtime with proper calling convention support
- More complete SDK stubs (GX, VI, AI, DSP initialization)
- Placeholder graphics output (e.g., via `minifb` or `softbuffer`) to visualize GX calls
- Floating-point instruction support
- Control flow analysis and multi-function recompilation

### Long-Term Vision
- Support full game boots to menu or in-game with basic rendering
- High-fidelity GX emulation using `wgpu` for modern GPU acceleration
- Audio stubs progressing toward real DSP emulation
- Input handling (SDL2 or similar)
- Modding support: easy hooking, patching, and asset replacement
- Multiple game compatibility with per-game configuration
- Potential exploration of recompiled binaries running back on real GameCube hardware (via Rust → PowerPC cross-compilation)
- Community contributions and shared symbol databases (integrating decomp efforts)

## Project Structure

The codebase is deliberately modular to keep concerns separated and make iterative progress sustainable:

GCRecomp/
├── game/                          # Standalone binary crate (the final game.exe/game)
│   ├── src/
│   │   ├── main.rs                # Entry point for the standalone game binary
│   │   └── recompiled.rs          # Auto-generated Rust from the recompiler
│   └── Cargo.toml
├── src/                           # Main library crate (gcrecomp)
│   ├── recompiler/                # Parsing, decoding, code generation
│   │   ├── parser.rs
│   │   ├── decoder.rs
│   │   └── codegen.rs
│   ├── runtime/                   # CPU context, memory, SDK stubs
│   │   ├── context.rs
│   │   └── sdk.rs
│   ├── lib.rs                     # Library crate entry point
│   └── main.rs                    # CLI for testing the recompiler
├── tests/                         # Test DOLs and Ghidra exports
├── docs/                          # Architecture, game support notes, coding rules
├── scripts/                       # Automation helpers (Ghidra scripting, etc.)
├── Cargo.toml                     # Workspace/root configuration
└── README.md

## Why This Project is Cool

- Combines deep reverse engineering with modern compiler techniques
- Pushes Rust into a domain usually dominated by C/C++
- Potential to breathe new life into GameCube classics with native performance and modding
- Fully open-source, no distributed ROMs — purely a technical preservation tool
- Cross-platform from day one

If you're into low-level programming, game preservation, or just love seeing impossible-sounding projects come to life, star/watch the repo and follow along!

Contributions, ideas, and moral support are very welcome as this grows.

**License**: MIT
