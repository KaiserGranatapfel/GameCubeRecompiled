# Development Guide

This guide is for developers who want to contribute to or extend GCRecomp.

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo
- Git
- (Optional) Ghidra for testing
- (Optional) Python 3.8+ for ReOxide

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/GCRecomp.git
cd GCRecomp

# Build in development mode
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Project Structure

```
GCRecomp/
├── gcrecomp-core/          # Core recompiler library
│   ├── src/
│   │   ├── recompiler/    # Main recompiler logic
│   │   │   ├── parser.rs  # DOL file parsing
│   │   │   ├── decoder.rs # Instruction decoding
│   │   │   ├── analysis/  # Static analysis
│   │   │   ├── codegen.rs # Code generation
│   │   │   └── pipeline.rs # Main pipeline
│   │   └── runtime/       # Runtime support
│   └── tests/             # Unit tests
├── gcrecomp-cli/          # Command-line interface
├── gcrecomp-runtime/      # Runtime system
├── gcrecomp-ui/           # Graphical interface
└── game/                  # Generated game code
```

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
```

### 2. Make Changes

Follow the code style guidelines:
- Use `cargo fmt` before committing
- Run `cargo clippy` to check for issues
- Write tests for new functionality
- Update documentation

### 3. Test Your Changes

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration
```

### 4. Commit Changes

```bash
git add .
git commit -m "Description of changes"
```

Use clear, descriptive commit messages:
- Start with a verb (Add, Fix, Update, etc.)
- Be specific about what changed
- Reference issue numbers if applicable

### 5. Push and Create PR

```bash
git push origin feature/your-feature-name
```

Then create a pull request on GitHub.

## Code Style

### Formatting

Always run `cargo fmt` before committing:

```bash
cargo fmt
```

### Linting

Run `cargo clippy` and fix all warnings:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Naming Conventions

- **Functions**: `snake_case`
- **Types**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Modules**: `snake_case`

### Documentation

Document all public APIs:

```rust
/// Brief description.
///
/// Detailed description with examples.
///
/// # Arguments
/// * `param` - Description
///
/// # Returns
/// Description of return value
///
/// # Errors
/// When this function returns an error
///
/// # Examples
/// ```
/// example_code();
/// ```
pub fn function(param: Type) -> Result<ReturnType> {
    // ...
}
```

## Testing

### Unit Tests

Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function() {
        // Test code
    }
}
```

### Integration Tests

Place integration tests in `tests/` directory:

```rust
// tests/integration_test.rs
use gcrecomp_core::*;

#[test]
fn test_integration() {
    // Test code
}
```

### Test Coverage

Aim for high test coverage, especially for:
- Core algorithms (decoder, analyzer)
- Error handling paths
- Edge cases

## Adding New Features

### 1. New Instruction Support

1. Add instruction to decoder:
   - Update `InstructionType` enum if needed
   - Add decoding logic in `decoder.rs`
   - Add operand parsing

2. Add code generation:
   - Update `codegen.rs` with instruction-specific handler
   - Test with sample instructions

3. Add tests:
   - Unit tests for decoding
   - Integration tests for codegen

### 2. New Analysis Pass

1. Create new module in `analysis/`
2. Implement analysis logic
3. Integrate into pipeline
4. Add tests
5. Update documentation

### 3. Runtime Improvements

1. Add functionality to `gcrecomp-runtime`
2. Update SDK stubs if needed
3. Test with recompiled code
4. Document changes

## Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run --bin gcrecomp-cli -- [args]
```

### Debug Specific Modules

```bash
RUST_LOG=gcrecomp_core::recompiler=debug cargo run
```

### Use Debugger

```bash
# Build with debug symbols
cargo build

# Attach debugger
gdb target/debug/gcrecomp-cli
# or
lldb target/debug/gcrecomp-cli
```

## Performance Profiling

### Using `perf` (Linux)

```bash
cargo build --release
perf record ./target/release/gcrecomp-cli [args]
perf report
```

### Using `cargo flamegraph`

```bash
cargo install flamegraph
cargo flamegraph --bin gcrecomp-cli -- [args]
```

## Memory Profiling

### Using `valgrind` (Linux)

```bash
valgrind --leak-check=full ./target/debug/gcrecomp-cli [args]
```

### Using `heaptrack` (Linux)

```bash
heaptrack ./target/release/gcrecomp-cli [args]
heaptrack_gui heaptrack.gcrecomp-cli.*.gz
```

## Common Tasks

### Adding a Dependency

1. Add to `Cargo.toml`:
```toml
[dependencies]
new-crate = "1.0"
```

2. Use in code:
```rust
use new_crate::*;
```

3. Update `Cargo.lock`:
```bash
cargo update
```

### Updating Dependencies

```bash
cargo update
cargo audit  # Check for security issues
```

### Building Documentation

```bash
# Build docs
cargo doc --no-deps

# Open in browser
cargo doc --open
```

## CI/CD

The project uses GitHub Actions for CI/CD. See `.github/workflows/` for configuration.

### Local CI Checks

Run CI checks locally:

```bash
# Format check
cargo fmt --all -- --check

# Clippy
cargo clippy --all-targets --all-features -- -D warnings

# Tests
cargo test --all

# Build
cargo build --all --release
```

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag: `git tag v0.1.0`
4. Push tag: `git push origin v0.1.0`
5. GitHub Actions will create release automatically

## Getting Help

- Read [CONTRIBUTING.md](../CONTRIBUTING.md)
- Check [ARCHITECTURE.md](ARCHITECTURE.md) for design details
- Review [API.md](API.md) for API reference
- Ask questions in GitHub Discussions
- Open an issue for bugs or feature requests

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [PowerPC Architecture](https://en.wikipedia.org/wiki/PowerPC)
- [GameCube Documentation](https://www.gc-forever.com/)

