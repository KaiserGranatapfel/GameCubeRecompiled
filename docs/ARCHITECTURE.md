# GCRecomp Architecture

This document describes the high-level architecture of GCRecomp, including its components, data flow, and design decisions.

## Overview

GCRecomp is a static recompiler that translates GameCube PowerPC binaries (DOL files) into optimized Rust code. The system consists of several stages that work together to produce native, executable Rust code.

## System Architecture

```
┌─────────────┐
│   DOL File  │
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│  Parser Module  │  Extracts sections, headers, metadata
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│ Ghidra Analysis │  Function discovery, type inference, symbols
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│  Decoder Module │  PowerPC instruction decoding
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│ Analysis Module │  CFG, data flow, type inference
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│  Code Generator │  Rust code generation
└──────┬──────────┘
       │
       ▼
┌─────────────────┐
│   Validator     │  Code validation
└──────┬──────────┘
       │
       ▼
┌─────────────┐
│  Rust Code  │
└─────────────┘
```

## Core Components

### 1. Parser (`gcrecomp-core/src/recompiler/parser.rs`)

**Purpose**: Parse DOL file format and extract executable sections.

**Responsibilities**:
- Read DOL file header
- Extract section metadata (address, size, type)
- Load section data into memory
- Validate file structure

**Key Data Structures**:
- `DolFile`: Represents parsed DOL file
- `Section`: Individual section information

### 2. Decoder (`gcrecomp-core/src/recompiler/decoder.rs`)

**Purpose**: Decode PowerPC instructions from binary format.

**Responsibilities**:
- Parse 32-bit instruction words
- Identify instruction types (arithmetic, branch, load/store, etc.)
- Extract operands (registers, immediates, addresses)
- Track instruction addresses

**Key Data Structures**:
- `Instruction`: Decoded instruction with operands
- `DecodedInstruction`: Instruction with address metadata
- `InstructionType`: Enum of instruction categories
- `Operand`: Variant type for different operand types

**Memory Optimizations**:
- `#[repr(u8)]` for enums (1 byte instead of 4-8)
- `SmallVec` for operand lists (avoids heap allocation for small lists)
- `#[repr(packed)]` for instruction structs

### 3. Analysis Module (`gcrecomp-core/src/recompiler/analysis/`)

**Purpose**: Perform static analysis on decoded instructions.

#### 3.1 Control Flow Analysis (`control_flow.rs`)

**Responsibilities**:
- Build Control Flow Graph (CFG)
- Identify basic blocks
- Detect loops and back edges
- Analyze branch targets

**Key Data Structures**:
- `ControlFlowGraph`: Graph of basic blocks
- `BasicBlock`: Sequence of instructions with single entry/exit
- `Edge`: Connection between blocks (conditional/unconditional)
- `Loop`: Loop information (header, body, exits)

#### 3.2 Data Flow Analysis (`data_flow.rs`)

**Responsibilities**:
- Build def-use chains
- Perform live variable analysis
- Identify dead code
- Track register dependencies

**Key Data Structures**:
- `DefUseChain`: Definition-use relationships
- `Definition`: Where a register is written
- `Use`: Where a register is read
- `LiveVariableAnalysis`: Live variable sets per block

#### 3.3 Type Inference (`type_inference.rs`)

**Responsibilities**:
- Infer types for registers and variables
- Track type information through operations
- Identify pointer types
- Detect integer sizes

**Key Data Structures**:
- `TypeInfo`: Type information (integer, pointer, void, etc.)
- `InferredType`: Type with confidence level

### 4. Code Generator (`gcrecomp-core/src/recompiler/codegen.rs`)

**Purpose**: Generate Rust code from analyzed instructions.

**Responsibilities**:
- Translate PowerPC instructions to Rust
- Generate function signatures
- Handle register allocation
- Optimize generated code
- Generate function dispatcher

**Key Features**:
- Instruction-specific code generation
- Constant folding optimization
- Register value tracking
- Function call handling
- Error recovery with stub generation

**Code Generation Strategy**:
1. Generate function signature (name, parameters, return type)
2. Initialize runtime context
3. Translate each instruction to Rust code
4. Handle control flow (branches, calls)
5. Return value handling

### 5. Pipeline (`gcrecomp-core/src/recompiler/pipeline.rs`)

**Purpose**: Orchestrate the complete recompilation process.

**Responsibilities**:
- Coordinate all stages
- Manage data flow between stages
- Handle errors and recovery
- Generate final output

**Pipeline Stages**:
1. Ghidra Analysis
2. Instruction Decoding
3. Control Flow Analysis
4. Data Flow Analysis
5. Type Inference
6. Code Generation
7. Validation
8. Output

### 6. Runtime System (`gcrecomp-runtime/`)

**Purpose**: Provide runtime support for recompiled code.

**Components**:
- **Memory Management**: RAM, VRAM, ARAM emulation
- **CPU Context**: Register state, condition registers
- **Graphics**: GX command processing, rendering
- **Input**: Controller handling
- **SDK Stubs**: GameCube SDK function implementations

## Data Flow

### Instruction Processing Flow

```
Binary Instruction (32 bits)
    ↓
Decode → Instruction Type + Operands
    ↓
Analysis → CFG, Data Flow, Types
    ↓
Code Generation → Rust Code
    ↓
Validation → Syntax Check
    ↓
Output → Rust Source File
```

### Function Processing Flow

```
Ghidra Function Metadata
    ↓
Map Instructions to Function (by address)
    ↓
Build CFG for Function
    ↓
Analyze Data Flow
    ↓
Generate Function Code
    ↓
Add to Function Dispatcher
```

## Memory Optimizations

### Enum Size Reduction
- Use `#[repr(u8)]` for enums that fit in 1 byte
- Saves 3-7 bytes per enum instance on 64-bit systems

### Struct Packing
- Use `#[repr(packed)]` to minimize padding
- Reduces memory usage by 10-20%

### Small Collections
- Use `SmallVec` for small, frequently allocated collections
- Avoids heap allocation for common cases
- Reduces allocation overhead

### Bit Sets
- Use `BitVec` for sets of registers/block IDs
- 1 bit per element instead of 8+ bytes
- Significant memory savings for large sets

## Error Handling

### Error Types
- Custom error types using `thiserror`
- Zero-cost error handling
- Detailed error messages

### Error Recovery
- Continue processing on non-fatal errors
- Generate stub functions for failed codegen
- Log warnings for debugging

## Design Decisions

### Why Rust?
- Memory safety without garbage collection
- Zero-cost abstractions
- Excellent performance
- Strong type system

### Why Static Recompilation?
- Better performance than emulation
- Native code execution
- Easier debugging
- Potential for optimization

### Why Ghidra Integration?
- Industry-standard reverse engineering tool
- Function discovery and analysis
- Type information recovery
- Symbol resolution

### Why IR (Intermediate Representation)?
- Separation of concerns
- Multiple optimization passes
- Easier to test and debug
- Future extensibility

## Future Architecture Considerations

### Planned Improvements
- More aggressive optimizations
- Better register allocation
- Loop optimization
- Function inlining
- SIMD instruction support

### Scalability
- Parallel processing for large binaries
- Incremental recompilation
- Caching of analysis results
- Streaming for very large files

## Related Documentation

- [API.md](API.md) - API reference
- [DEVELOPMENT.md](DEVELOPMENT.md) - Development guide
- [README.md](../README.md) - Project overview

