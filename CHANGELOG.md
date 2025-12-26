# Changelog

All notable changes to GCRecomp will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Complete PowerPC instruction decoding support
- Advanced control flow analysis with loop detection
- Data flow analysis with def-use chains
- Type inference engine
- IR (Intermediate Representation) optimization passes
- Memory-optimized data structures (SmallVec, BitVec)
- Comprehensive error handling with thiserror
- Ghidra integration with ReOxide support
- Function call handling and dispatcher
- Progress reporting in pipeline
- Code validation system

### Changed
- Improved instruction-specific code generation
- Enhanced floating-point operation support
- Better condition register operations
- Optimized memory usage (20-30% reduction)

### Fixed
- Function-to-instruction mapping
- Pipeline backend parameter passing
- Instruction address tracking

## [0.1.0] - 2024-XX-XX

### Added
- Initial release
- DOL file parsing
- PowerPC instruction decoder
- Basic code generation
- CLI interface
- Runtime system foundation

[Unreleased]: https://github.com/yourusername/GCRecomp/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/yourusername/GCRecomp/releases/tag/v0.1.0

