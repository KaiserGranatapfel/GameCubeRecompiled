# GCRecomp Scripts

## ghidra_export.py

This Python script is used by GCRecomp to export analysis data from Ghidra.

### Usage

The script is automatically executed by GCRecomp when analyzing a DOL file. It can also be run manually in Ghidra:

1. Open your DOL file in Ghidra
2. Run analysis (Analysis → Auto Analyze)
3. Open Script Manager (Window → Script Manager)
4. Run `ghidra_export.py`

### Output

The script generates three JSON files in the directory specified by `GHIDRA_EXPORT_DIR` environment variable (default: `/tmp/ghidra_export`):

- **functions.json**: All function information including parameters, return types, local variables, and basic blocks
- **symbols.json**: All symbols (functions, data, labels) with addresses and namespaces
- **decompiled.json**: Decompiled C code for each function

### Requirements

- Ghidra with Python scripting enabled
- Analyzed binary (run Auto Analyze first)

### Customization

You can modify the script to export additional information:
- Add more function metadata
- Export data references
- Include cross-references
- Export type information

