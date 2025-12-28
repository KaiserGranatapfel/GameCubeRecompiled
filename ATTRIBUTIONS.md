# Attributions and Credits

This file lists all third-party code, libraries, and inspirations used in GCRecomp.

## Major Inspirations

### N64Recomp
- **Repository**: https://github.com/N64Recomp/N64Recomp
- **License**: See N64Recomp repository for license information
- **Usage**: 
  - Gyro control implementation heavily influenced by and adapted from N64Recomp
  - Static recompilation approach inspired by N64Recomp's methodology
  - Code organization and structure patterns influenced by N64Recomp
- **Files**:
  - `gcrecomp-runtime/src/input/gyro.rs` - Gyro implementation adapted from N64Recomp
  - `gcrecomp-runtime/src/input/switch_pro.rs` - Switch Pro Controller support influenced by N64Recomp

## Third-Party Libraries

### Rust Standard Library and Crates

#### Core Dependencies
- **anyhow** (https://github.com/dtolnay/anyhow) - Error handling
  - License: MIT OR Apache-2.0
- **thiserror** (https://github.com/dtolnay/thiserror) - Error types
  - License: MIT OR Apache-2.0
- **serde** (https://github.com/serde-rs/serde) - Serialization framework
  - License: MIT OR Apache-2.0
- **serde_json** (https://github.com/serde-rs/json) - JSON support
  - License: MIT OR Apache-2.0
- **log** (https://github.com/rust-lang/log) - Logging facade
  - License: MIT OR Apache-2.0

#### Binary Parsing
- **goblin** (https://github.com/m4b/goblin) - Binary parsing library
  - License: MIT

#### CLI
- **clap** (https://github.com/clap-rs/clap) - Command-line argument parser
  - License: MIT OR Apache-2.0
- **indicatif** (https://github.com/console-rs/indicatif) - Progress bars
  - License: MIT OR Apache-2.0

#### UI
- **iced** (https://github.com/iced-rs/iced) - GUI framework
  - License: MIT
- **winit** (https://github.com/rust-windowing/winit) - Window management
  - License: Apache-2.0
- **wgpu** (https://github.com/gfx-rs/wgpu) - Graphics API
  - License: Apache-2.0 OR MIT

#### Runtime
- **minifb** (https://github.com/emoon/rust_minifb) - Minimal windowing
  - License: MIT

#### Memory Optimization
- **smallvec** (https://github.com/servo/rust-smallvec) - Small vector optimization
  - License: MIT OR Apache-2.0
- **bitvec** (https://github.com/bitvecto-rs/bitvec) - Bit-level collections
  - License: MIT

#### Input/Controller Support
- **gilrs** (https://gitlab.com/gilrs-project/gilrs) - Cross-platform gamepad support
  - License: Apache-2.0 OR MIT
- **sdl2** (https://github.com/Rust-SDL2/rust-sdl2) - SDL2 bindings
  - License: MIT
- **hidapi** (https://github.com/ruabmbua/hidapi-rs) - HID API bindings
  - License: MIT OR Apache-2.0

#### Audio
- **cpal** (https://github.com/RustAudio/cpal) - Cross-platform audio library
  - License: Apache-2.0 OR MIT

#### Text Processing
- **regex** (https://github.com/rust-lang/regex) - Regular expressions
  - License: MIT OR Apache-2.0

#### Utilities
- **which** (https://github.com/harryfei/which-rs) - Find executables in PATH
  - License: MIT

## External Tools and Services

### Ghidra
- **Website**: https://ghidra-sre.org/
- **License**: Apache-2.0
- **Usage**: Binary analysis and reverse engineering
- **Note**: GCRecomp integrates with Ghidra for enhanced analysis but does not include Ghidra itself

### ReOxide
- **Repository**: https://github.com/tr3x/reoxide (if available)
- **Usage**: Python tool for enhanced Ghidra integration
- **Note**: Auto-installed via pipx/pip if not present

## Code Adaptations

### Gyro Controls
The gyro control implementation in `gcrecomp-runtime/src/input/gyro.rs` is heavily influenced by N64Recomp's gyro implementation. The code has been:
- Translated from the original language to Rust
- Adapted for GameCube-specific requirements
- Enhanced with additional features (calibration, sensitivity, dead zones)

### Controller Support
Controller input handling patterns were influenced by:
- N64Recomp's input system
- Dolphin emulator's controller mapping approach
- Modern gamepad library best practices

## Documentation and References

### PowerPC Architecture
- PowerPC Architecture documentation
- GameCube programming guides
- Official Nintendo GameCube SDK documentation (referenced, not included)

### Reverse Engineering
- Ghidra documentation and tutorials
- Static recompilation research papers
- Binary analysis techniques from various decompilation projects

## License Compatibility

All third-party dependencies used in GCRecomp are compatible with the project's CC0-1.0 license. Where code has been adapted from other projects:
- Proper attribution is provided
- Original licenses are respected
- Adaptations are clearly marked

## Contributing

If you use code from GCRecomp in your project, please:
1. Respect the CC0-1.0 license
2. Provide attribution to GCRecomp
3. If you adapt code that was originally from N64Recomp, also attribute N64Recomp

## Contact

For questions about attributions or licensing, please open an issue on the GitHub repository.

