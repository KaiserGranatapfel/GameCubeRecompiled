//! Analysis Module
//!
//! This module provides static analysis capabilities for PowerPC code,
//! including control flow analysis, data flow analysis, type inference,
//! pointer analysis, struct detection, and more.

pub mod control_flow;
pub mod data_flow;
pub mod inter_procedural;
pub mod loop_analysis;
pub mod pointer;
pub mod structs;
pub mod type_inference;

// Re-export commonly used types
pub use control_flow::{BasicBlock, ControlFlowGraph, Edge, EdgeType};
pub use data_flow::{DataFlowAnalysis, DefUseChain};
pub use type_inference::{InferredType, TypeInferenceEngine};

// Analysis data structures
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionMetadata {
    pub address: u32,
    pub name: String,
    pub size: u32,
    pub calling_convention: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<TypeInfo>,
    pub local_variables: Vec<VariableInfo>,
    pub basic_blocks: Vec<control_flow::BasicBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub type_info: TypeInfo,
    pub register: Option<u8>,
    pub stack_offset: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableInfo {
    pub name: String,
    pub type_info: TypeInfo,
    pub stack_offset: i32,
    pub scope_start: u32,
    pub scope_end: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeInfo {
    Void,
    Integer {
        signed: bool,
        size: u8,
    },
    Pointer {
        pointee: Box<TypeInfo>,
    },
    Struct {
        name: String,
        fields: Vec<FieldInfo>,
    },
    Array {
        element: Box<TypeInfo>,
        size: Option<usize>,
    },
    Function {
        params: Vec<TypeInfo>,
        return_type: Box<TypeInfo>,
    },
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInfo {
    pub name: String,
    pub type_info: TypeInfo,
    pub offset: u32,
}

// ControlFlowGraph, Edge, and EdgeType are defined in control_flow.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowAnalysis {
    pub function_address: u32,
    pub def_use_chains: HashMap<u32, DefUseChain>, // Instruction address -> def-use chain
    pub live_variables: HashMap<u32, Vec<u8>>,     // Instruction address -> live registers
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefUseChain {
    pub definition: u32, // Instruction address where variable is defined
    pub uses: Vec<u32>,  // Instruction addresses where variable is used
    pub register: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInformation {
    pub structs: HashMap<String, StructInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub function_signatures: HashMap<u32, FunctionSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructInfo {
    pub name: String,
    pub size: u32,
    pub alignment: u32,
    pub fields: Vec<FieldInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub value: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub address: u32,
    pub name: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<TypeInfo>,
    pub calling_convention: String,
}

// ControlFlowGraph methods are defined in control_flow.rs
