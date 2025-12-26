# Test Fixtures

This directory contains test fixtures for GCRecomp integration tests.

## Legal Notice

**IMPORTANT**: This directory should NOT contain any copyrighted GameCube game files, ROMs, ISOs, or DOL files.

Test fixtures should only include:
- Synthetic test data
- Minimal valid DOL file structures (if legally created)
- Test instruction sequences
- Mock data for testing

## Creating Test Fixtures

If you need to create test fixtures:

1. **Synthetic Data Only**: Create minimal, synthetic test data
2. **No Copyrighted Content**: Do not include any actual game files
3. **Documentation**: Document what each fixture tests
4. **Legal Compliance**: Ensure all fixtures comply with the [EULA](../EULA.md)

## Example Test Fixtures

- `minimal_dol_header.bin`: Minimal valid DOL header structure
- `test_instructions.bin`: Synthetic instruction sequences for testing
- `mock_section.bin`: Mock section data for parser tests

## Usage

Test fixtures are used in integration tests:

```rust
let fixture_path = PathBuf::from("tests/fixtures/minimal_dol_header.bin");
let dol_file = DolFile::parse(&fixture_path)?;
```

