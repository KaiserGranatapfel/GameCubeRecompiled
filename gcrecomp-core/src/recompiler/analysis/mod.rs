pub mod control_flow;
pub mod data_flow;
pub mod inter_procedural;
pub mod loop_analysis;
pub mod type_inference;

/// Type information for decompiled/recompiled code
#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    /// Unknown type (not yet inferred)
    Unknown,
    /// Void type
    Void,
    /// Integer type with signedness and size
    Integer { signed: bool, size: u8 },
    /// Pointer to another type
    Pointer { pointee: Box<TypeInfo> },
    /// Floating point type
    Float { size: u8 },
    /// Array type
    Array { element: Box<TypeInfo>, size: usize },
    /// Structure type
    Struct {
        name: String,
        fields: Vec<(String, TypeInfo)>,
    },
}

/// Function metadata extracted from analysis
#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    /// Function start address
    pub address: u32,
    /// Function name
    pub name: String,
    /// Function size in bytes
    pub size: u32,
    /// Calling convention (e.g., "cdecl", "fastcall")
    pub calling_convention: String,
    /// Function parameters
    pub parameters: Vec<ParameterInfo>,
    /// Return type
    pub return_type: Option<TypeInfo>,
    /// Local variables
    pub local_variables: Vec<VariableInfo>,
    /// Basic block addresses
    pub basic_blocks: Vec<u32>,
}

/// Parameter information
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub type_info: TypeInfo,
    /// Register used (if any)
    pub register: Option<u8>,
    /// Stack offset (if stack parameter)
    pub stack_offset: i32,
}

/// Variable information
#[derive(Debug, Clone)]
pub struct VariableInfo {
    /// Variable name
    pub name: String,
    /// Variable type
    pub type_info: TypeInfo,
    /// Stack offset
    pub stack_offset: i32,
    /// Scope start address
    pub scope_start: u32,
    /// Scope end address
    pub scope_end: u32,
}
