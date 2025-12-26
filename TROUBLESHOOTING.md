# Troubleshooting Guide

Common issues and solutions when using GCRecomp.

## Build Issues

### "error: linker 'cc' not found"
**Problem**: Missing C compiler required by Rust.

**Solution**:
- **Linux**: Install `build-essential` (Ubuntu/Debian) or `gcc` (other distros)
- **macOS**: Install Xcode Command Line Tools: `xcode-select --install`
- **Windows**: Install Visual Studio Build Tools or use the `msvc` toolchain

### "error: failed to compile"
**Problem**: Compilation errors, often due to dependency issues.

**Solution**:
```bash
# Update Rust toolchain
rustup update

# Clean build artifacts
cargo clean

# Update dependencies
cargo update

# Rebuild
cargo build --release
```

### "error: could not find 'Cargo.toml'"
**Problem**: Not in the project root directory.

**Solution**: Navigate to the project root:
```bash
cd GCRecomp
```

## Runtime Issues

### "Ghidra not found"
**Problem**: Ghidra installation not detected.

**Solution**:
1. Install Ghidra from [https://ghidra-sre.org/](https://ghidra-sre.org/)
2. Set environment variable:
   ```bash
   export GHIDRA_INSTALL_DIR=/path/to/ghidra
   ```
3. Or ensure Ghidra is in your PATH

### "ReOxide installation failed"
**Problem**: Python or pip not available.

**Solution**:
1. Install Python 3.8 or later
2. Install pipx (recommended) or ensure pip is available
3. Manually install ReOxide:
   ```bash
   pipx install reoxide
   # or
   pip install reoxide
   ```

### "DOL file parsing failed"
**Problem**: Invalid or corrupted DOL file.

**Solution**:
1. Verify the DOL file is valid (check file size, header)
2. Ensure the file is from a legally owned physical disc
3. Try re-dumping the disc if possible
4. Check file permissions

### "Out of memory" errors
**Problem**: Large DOL files or insufficient system memory.

**Solution**:
1. Close other applications
2. Use release build: `cargo build --release`
3. Process smaller sections if possible
4. Increase system swap space

## Code Generation Issues

### "Function generation failed"
**Problem**: Unable to generate code for a function.

**Solution**:
1. Check the function has valid instructions
2. Review error logs for specific issues
3. The system will generate stub functions as fallback
4. Report the issue with function address and error message

### "Instruction decoding failed"
**Problem**: Unknown or invalid PowerPC instruction.

**Solution**:
1. This may indicate an edge case not yet supported
2. Check if the instruction is documented in PowerPC specification
3. Report the issue with instruction bytes and address
4. The system will log a warning and continue

### "Validation errors in generated code"
**Problem**: Generated Rust code has syntax errors.

**Solution**:
1. Check the validation error message
2. Review the generated code around the error location
3. This may indicate a bug in the code generator
4. Report the issue with the validation error and code snippet

## Performance Issues

### "Recompilation is very slow"
**Problem**: Large DOL files take a long time to process.

**Solution**:
1. Use release build: `cargo build --release`
2. Process in smaller chunks if possible
3. Disable optional analysis passes
4. Use faster hardware or more CPU cores

### "High memory usage"
**Problem**: GCRecomp uses too much memory.

**Solution**:
1. This is expected for large binaries
2. Use release build (more memory efficient)
3. Close other applications
4. Process smaller sections sequentially

## Platform-Specific Issues

### Linux: "Permission denied"
**Problem**: Cannot execute binaries or access files.

**Solution**:
```bash
# Make executable
chmod +x target/release/gcrecomp-cli

# Check file permissions
ls -l target/release/
```

### macOS: "cannot be opened because the developer cannot be verified"
**Problem**: macOS Gatekeeper blocking execution.

**Solution**:
```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine target/release/gcrecomp-cli

# Or allow in System Preferences > Security & Privacy
```

### Windows: "The program can't start because MSVCP140.dll is missing"
**Problem**: Missing Visual C++ Redistributable.

**Solution**:
1. Download and install [Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
2. Restart your computer
3. Try running again

## Getting Help

If you're still experiencing issues:

1. **Check existing issues**: Search [GitHub Issues](https://github.com/yourusername/GCRecomp/issues) for similar problems
2. **Check logs**: Enable verbose logging:
   ```bash
   RUST_LOG=debug cargo run --release --bin gcrecomp-cli -- [args]
   ```
3. **Create an issue**: Open a new issue with:
   - Description of the problem
   - Steps to reproduce
   - Error messages
   - System information (OS, Rust version, etc.)
   - Relevant logs

## Common Error Messages

### "No such file or directory"
- Check file paths are correct
- Use absolute paths if relative paths fail
- Check file permissions

### "Connection refused" (Ghidra)
- Ensure Ghidra is running
- Check Ghidra port configuration
- Verify network connectivity

### "Out of bounds"
- Memory access error
- Check DOL file is valid
- Verify address calculations

### "Unsupported instruction"
- Instruction not yet implemented
- Check if it's a valid PowerPC instruction
- Report for future support

## Prevention Tips

1. **Always use release builds** for production
2. **Keep Rust updated**: `rustup update`
3. **Validate DOL files** before processing
4. **Read error messages carefully** - they often contain solutions
5. **Check documentation** before reporting issues
6. **Use version control** to track changes

For more help, see:
- [README.md](README.md) - General information
- [INSTALL.md](INSTALL.md) - Installation guide
- [CONTRIBUTING.md](CONTRIBUTING.md) - Development information
- [docs/](docs/) - Detailed documentation

