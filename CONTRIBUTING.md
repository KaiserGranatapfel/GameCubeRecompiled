# Contributing to GCRecomp

Thank you for your interest in contributing to GCRecomp! This project thrives on community collaboration to advance game preservation, recompilation tech, and Rust-based tools. We welcome contributions from everyone, regardless of experience level, to make GCRecomp better.

Before contributing, please review the [EULA](EULA.md) and ensure your work aligns with clean-room reverse engineering—no use of proprietary Nintendo materials.

## Code of Conduct
This project adheres to the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). We expect all participants to create a welcoming, harassment-free environment. Report issues to the project maintainers via GitHub issues.

## How to Contribute
1. **Fork the Repository**: Click "Fork" on the GitHub page to create your own copy.
2. **Clone Your Fork**: `git clone https://github.com/yourusername/GCRecomp.git`
3. **Create a Branch**: Use descriptive names, e.g., `git checkout -b feature/new-instruction-support`
4. **Make Changes**: Follow the code quality standards below.
5. **Test Your Changes**: Run `cargo test` and ensure all tests pass.
6. **Commit**: Use clear messages, e.g., "Add support for new PowerPC instruction"
7. **Push and Open a Pull Request**: Push to your fork and submit a PR to the main repo. Describe your changes, reference any issues, and explain why it's valuable.

We aim to review PRs within 7 days. If your PR addresses an open issue, reference it (e.g., "Fixes #123").

## Areas for Contribution
As outlined in README.md:
- Additional instruction support (e.g., edge-case PowerPC ops)
- Runtime improvements (e.g., SDK stubs, graphics/audio emulation)
- Documentation enhancements (e.g., examples, API docs)
- Bug fixes and optimizations (e.g., memory efficiency)
- New features from the roadmap (e.g., wgpu integration)

Other ideas: Accessibility improvements, CI/CD scripts, or community tools.

## Code Quality Standards
- **Rust Best Practices**: Use idiomatic Rust, handle errors with `thiserror`, and leverage crates like `SmallVec` for efficiency.
- **Documentation**: Add inline comments for complex logic; update API docs with `cargo doc`.
- **Testing**: Include unit/integration tests for new features; aim for high coverage.
- **Style**: Follow existing formatting (use `cargo fmt`); no unsafe code without justification.
- **Commits**: Keep them atomic and descriptive.
- **Legal Compliance**: Ensure contributions are original and comply with clean-room RE.

## Development Setup

### Prerequisites
- Rust 1.70 or later (check with `rustc --version`)
- Cargo (comes with Rust)
- Git
- (Optional) Ghidra for advanced analysis

### Building
```bash
# Clone the repository
git clone https://github.com/yourusername/GCRecomp.git
cd GCRecomp

# Build the project
cargo build

# Run tests
cargo test

# Format code
cargo fmt

# Run linter
cargo clippy
```

### Running Tests
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test suite
cargo test --package gcrecomp-core

# Run integration tests
cargo test --test integration
```

### Code Review Process
1. All PRs require at least one maintainer review
2. CI must pass (tests, linting, formatting)
3. Code must follow project style guidelines
4. Documentation must be updated for new features
5. Tests must be included for new functionality

## First-Time Contributors
If you're new to Rust or GitHub, check out issues labeled "good first issue" or "help wanted." We're here to help—ask questions in discussions!

## Recognition
All contributors are acknowledged in README.md. By contributing, you agree to dedicate your work to the public domain under CC0-1.0.

## Security
If you discover a security vulnerability, please do NOT open a public issue. Instead, see [SECURITY.md](SECURITY.md) for reporting instructions.

Thanks for helping preserve gaming history! 🚀

