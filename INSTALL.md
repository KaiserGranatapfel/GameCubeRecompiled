# Installation Guide

This guide will help you install and set up GCRecomp on your system.

## Prerequisites

### Required
- **Rust 1.70 or later**: GCRecomp is written in Rust and requires the Rust toolchain
- **Cargo**: Comes bundled with Rust
- **Git**: For cloning the repository

### Optional
- **Ghidra**: For advanced binary analysis (recommended for best results)
- **Python 3.8+**: Required if using Ghidra integration
- **pipx or pip**: For installing ReOxide (Python-based Ghidra tool)

## Installation Methods

### Method 1: From Source (Recommended)

#### Step 1: Install Rust
If you don't have Rust installed, use rustup:

```bash
# On Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# On Windows
# Download and run rustup-init.exe from https://rustup.rs/
```

After installation, restart your terminal or run:
```bash
source $HOME/.cargo/env
```

Verify installation:
```bash
rustc --version
cargo --version
```

#### Step 2: Clone the Repository
```bash
git clone https://github.com/yourusername/GCRecomp.git
cd GCRecomp
```

#### Step 3: Build the Project
```bash
# Build in debug mode (faster compilation, slower runtime)
cargo build

# Or build in release mode (slower compilation, faster runtime)
cargo build --release
```

#### Step 4: Install CLI Tool (Optional)
```bash
# Install the CLI tool globally
cargo install --path gcrecomp-cli

# Or use directly from the project
cargo run --release --bin gcrecomp-cli -- --help
```

### Method 2: Using Pre-built Binaries

Download pre-built binaries from the [Releases](https://github.com/yourusername/GCRecomp/releases) page.

1. Download the appropriate binary for your platform
2. Extract the archive
3. Add to your PATH (optional but recommended)

```bash
# Linux/macOS
chmod +x gcrecomp-cli
sudo mv gcrecomp-cli /usr/local/bin/

# Windows
# Add the directory containing gcrecomp-cli.exe to your PATH
```

## Platform-Specific Instructions

### Linux

#### Ubuntu/Debian
```bash
# Install dependencies
sudo apt update
sudo apt install build-essential curl git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/yourusername/GCRecomp.git
cd GCRecomp
cargo build --release
```

#### Fedora/RHEL
```bash
# Install dependencies
sudo dnf install gcc curl git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/yourusername/GCRecomp.git
cd GCRecomp
cargo build --release
```

#### Arch Linux
```bash
# Install dependencies
sudo pacman -S base-devel curl git

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/yourusername/GCRecomp.git
cd GCRecomp
cargo build --release
```

### macOS

#### Using Homebrew
```bash
# Install dependencies
brew install rust git

# Clone and build
git clone https://github.com/yourusername/GCRecomp.git
cd GCRecomp
cargo build --release
```

#### Manual Installation
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/yourusername/GCRecomp.git
cd GCRecomp
cargo build --release
```

### Windows

#### Using Chocolatey
```powershell
# Install Rust
choco install rust

# Clone and build
git clone https://github.com/yourusername/GCRecomp.git
cd GCRecomp
cargo build --release
```

#### Manual Installation
1. Download and run [rustup-init.exe](https://rustup.rs/)
2. Open PowerShell or Command Prompt
3. Clone and build:
```powershell
git clone https://github.com/yourusername/GCRecomp.git
cd GCRecomp
cargo build --release
```

## Optional: Ghidra Setup

For advanced analysis features, install Ghidra:

1. Download Ghidra from [https://ghidra-sre.org/](https://ghidra-sre.org/)
2. Extract to a directory (e.g., `/opt/ghidra` on Linux, `C:\ghidra` on Windows)
3. Set `GHIDRA_INSTALL_DIR` environment variable:
   ```bash
   # Linux/macOS
   export GHIDRA_INSTALL_DIR=/opt/ghidra
   
   # Windows (PowerShell)
   $env:GHIDRA_INSTALL_DIR = "C:\ghidra"
   ```

GCRecomp will automatically install ReOxide when needed if Python is available.

## Verification

After installation, verify everything works:

```bash
# Check Rust version
rustc --version  # Should be 1.70 or later

# Check Cargo version
cargo --version

# Run tests
cargo test

# Check CLI help
cargo run --release --bin gcrecomp-cli -- --help
```

## Troubleshooting

### Common Issues

#### "command not found: cargo"
- Make sure Rust is installed: `rustc --version`
- Restart your terminal after installing Rust
- On Linux/macOS, run: `source $HOME/.cargo/env`

#### Build Errors
- Ensure you have the latest Rust version: `rustup update`
- Clean and rebuild: `cargo clean && cargo build`
- Check [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for more help

#### Ghidra Not Found
- Set `GHIDRA_INSTALL_DIR` environment variable
- Or install Ghidra and ensure it's in your PATH

For more help, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md) or open an issue on GitHub.

## Next Steps

After installation:
1. Read the [README.md](README.md) for usage instructions
2. Review the [EULA.md](EULA.md) for legal information
3. Check out [CONTRIBUTING.md](CONTRIBUTING.md) if you want to contribute
4. See [docs/](docs/) for detailed documentation

## Uninstallation

To uninstall GCRecomp:

```bash
# If installed via cargo install
cargo uninstall gcrecomp-cli

# If built from source, simply delete the repository
rm -rf GCRecomp  # Linux/macOS
# or
rmdir /s GCRecomp  # Windows
```

