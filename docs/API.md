# API Reference

This document provides a comprehensive API reference for GCRecomp's public interfaces.

## Core Library (`gcrecomp-core`)

### Recompiler Module

#### `RecompilationPipeline`

Main entry point for the recompilation process.

```rust
pub struct RecompilationPipeline;

impl RecompilationPipeline {
    pub fn recompile(dol_file: &DolFile, output_path: &str) -> Result<()>;
}
```

**Example**:
```rust
use gcrecomp_core::recompiler::pipeline::RecompilationPipeline;
use gcrecomp_core::recompiler::parser::DolFile;

let dol_file = DolFile::parse("game.dol")?;
RecompilationPipeline::recompile(&dol_file, "output.rs")?;
```

#### `DolFile`

Represents a parsed DOL file.

```rust
pub struct DolFile {
    pub path: PathBuf,
    pub sections: Vec<Section>,
    // ...
}

impl DolFile {
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<Self>;
}
```

#### `Instruction`

Represents a decoded PowerPC instruction.

```rust
pub struct Instruction {
    pub opcode: u8,
    pub instruction_type: InstructionType,
    pub operands: SmallVec<[Operand; 4]>,
    pub raw: u32,
}

impl Instruction {
    pub fn decode(word: u32, address: u32) -> Result<DecodedInstruction>;
}
```

#### `DecodedInstruction`

Instruction with address metadata.

```rust
#[repr(packed)]
pub struct DecodedInstruction {
    pub instruction: Instruction,
    pub address: u32,
    pub raw: u32,
}
```

### Analysis Module

#### `ControlFlowAnalyzer`

Builds and analyzes control flow graphs.

```rust
pub struct ControlFlowAnalyzer;

impl ControlFlowAnalyzer {
    pub fn build_cfg(
        instructions: &[DecodedInstruction],
        entry_address: u32
    ) -> Result<ControlFlowGraph>;
}
```

#### `DataFlowAnalyzer`

Performs data flow analysis.

```rust
pub struct DataFlowAnalyzer;

impl DataFlowAnalyzer {
    pub fn build_def_use_chains(
        instructions: &[DecodedInstruction]
    ) -> Vec<DefUseChain>;
    
    pub fn live_variable_analysis(
        cfg: &ControlFlowGraph
    ) -> LiveVariableAnalysis;
    
    pub fn eliminate_dead_code(
        instructions: &[DecodedInstruction],
        live: &LiveVariableAnalysis
    ) -> Vec<DecodedInstruction>;
}
```

### Code Generation

#### `CodeGenerator`

Generates Rust code from instructions.

```rust
pub struct CodeGenerator {
    // ...
}

impl CodeGenerator {
    pub fn new() -> Self;
    pub fn with_optimizations(self, optimize: bool) -> Self;
    pub fn generate_function(
        &mut self,
        metadata: &FunctionMetadata,
        instructions: &[DecodedInstruction]
    ) -> Result<String>;
}
```

### Ghidra Integration

#### `GhidraAnalysis`

Manages Ghidra analysis integration.

```rust
pub struct GhidraAnalysis {
    pub functions: Vec<FunctionInfo>,
    pub symbols: Vec<SymbolInfo>,
    // ...
}

impl GhidraAnalysis {
    pub fn analyze<P: AsRef<Path>>(
        dol_path: P,
        backend: GhidraBackend
    ) -> Result<Self>;
}
```

#### `GhidraBackend`

Backend selection for Ghidra analysis.

```rust
pub enum GhidraBackend {
    HeadlessCli,
    ReOxide,
}
```

## Runtime Library (`gcrecomp-runtime`)

### Memory Management

#### `Ram`

Main RAM emulation (24MB).

```rust
pub struct Ram {
    // ...
}

impl Ram {
    pub fn new() -> Self;
    pub fn read_u8(&self, address: u32) -> Result<u8>;
    pub fn read_u16(&self, address: u32) -> Result<u16>;
    pub fn read_u32(&self, address: u32) -> Result<u32>;
    pub fn write_u8(&mut self, address: u32, value: u8) -> Result<()>;
    pub fn write_u16(&mut self, address: u32, value: u16) -> Result<()>;
    pub fn write_u32(&mut self, address: u32, value: u32) -> Result<()>;
}
```

#### `VRam`

Video RAM emulation.

```rust
pub struct VRam {
    // ...
}

impl VRam {
    pub fn new() -> Self;
    // Similar interface to Ram
}
```

#### `ARam`

Audio RAM emulation.

```rust
pub struct ARam {
    // ...
}

impl ARam {
    pub fn new() -> Self;
    // Similar interface to Ram
}
```

### CPU Context

#### `CpuContext`

Represents CPU register state.

```rust
pub struct CpuContext {
    // ...
}

impl CpuContext {
    pub fn new() -> Self;
    pub fn get_register(&self, reg: u8) -> u32;
    pub fn set_register(&mut self, reg: u8, value: u32);
    pub fn get_fpr(&self, reg: u8) -> f64;
    pub fn set_fpr(&mut self, reg: u8, value: f64);
    pub fn get_cr_field(&self, field: u8) -> u8;
    pub fn set_cr_field(&mut self, field: u8, value: u8);
}
```

### Runtime

#### `Runtime`

Main runtime system.

```rust
pub struct Runtime {
    // ...
}

impl Runtime {
    pub fn new() -> Result<Self>;
    pub fn update(&mut self) -> Result<()>;
    pub fn ram(&self) -> &Ram;
    pub fn ram_mut(&mut self) -> &mut Ram;
    // ...
}
```

## Error Types

### `RecompilerError`

Main error type for recompiler operations.

```rust
#[derive(Error, Debug)]
pub enum RecompilerError {
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Decode error: {0}")]
    DecodeError(String),
    
    #[error("Analysis error: {0}")]
    AnalysisError(String),
    
    #[error("Code generation error: {0}")]
    CodeGenError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Ghidra error: {0}")]
    GhidraError(String),
}
```

## Type Definitions

### `InstructionType`

Categories of PowerPC instructions.

```rust
#[repr(u8)]
pub enum InstructionType {
    Arithmetic,
    Load,
    Store,
    Branch,
    Compare,
    Move,
    System,
    FloatingPoint,
    ConditionRegister,
    Shift,
    Rotate,
    Unknown,
}
```

### `Operand`

Instruction operands.

```rust
pub enum Operand {
    Register(u8),
    FpRegister(u8),
    Immediate(i16),
    Immediate32(i32),
    Address(u32),
    Condition(u8),
    SpecialRegister(u16),
    ShiftAmount(u8),
    Mask(u32),
}
```

### `TypeInfo`

Type information for variables and registers.

```rust
pub enum TypeInfo {
    Void,
    Integer { signed: bool, size: u8 },
    Pointer { pointee: Box<TypeInfo> },
    FloatingPoint { size: u8 },
    Unknown,
}
```

## Examples

### Basic Recompilation

```rust
use gcrecomp_core::recompiler::pipeline::RecompilationPipeline;
use gcrecomp_core::recompiler::parser::DolFile;

fn main() -> Result<()> {
    let dol_file = DolFile::parse("game.dol")?;
    RecompilationPipeline::recompile(&dol_file, "output.rs")?;
    Ok(())
}
```

### Custom Analysis

```rust
use gcrecomp_core::recompiler::decoder::Instruction;
use gcrecomp_core::recompiler::analysis::control_flow::ControlFlowAnalyzer;

let instructions = decode_all_instructions(&dol_file)?;
let cfg = ControlFlowAnalyzer::build_cfg(&instructions, 0x80000000)?;
// Analyze CFG...
```

### Runtime Usage

```rust
use gcrecomp_runtime::Runtime;

let mut runtime = Runtime::new()?;
runtime.update()?;
let ram = runtime.ram();
let value = ram.read_u32(0x80000000)?;
```

## Documentation

Generate full API documentation:

```bash
cargo doc --open
```

This will generate and open comprehensive API documentation in your browser.

