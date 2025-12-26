#!/usr/bin/env python3
"""
Ghidra script to export function and symbol information for GCRecomp
Run this script in Ghidra to export analysis data to JSON
"""

import json
import os
from ghidra.program.model.listing import FunctionManager
from ghidra.program.model.symbol import SymbolTable
from ghidra.program.model.address import AddressSet
from ghidra.program.model.lang import OperandType

def export_functions():
    """Export all function information"""
    functions = []
    func_manager = currentProgram.getFunctionManager()
    
    for func in func_manager.getFunctions(True):
        entry_point = func.getEntryPoint()
        body = func.getBody()
        
        # Get function signature
        signature = func.getSignature()
        calling_convention = func.getCallingConvention()
        if calling_convention:
            calling_convention = calling_convention.getName()
        else:
            calling_convention = "default"
        
        # Get parameters
        parameters = []
        for param in func.getParameters():
            param_type = param.getDataType()
            parameters.append({
                "name": param.getName(),
                "type": str(param_type),
                "offset": param.getStackOffset(),
            })
        
        # Get return type
        return_type = func.getReturnType()
        return_type_str = str(return_type) if return_type else None
        
        # Get local variables
        local_vars = []
        for var in func.getLocalVariables():
            local_vars.append({
                "name": var.getName(),
                "type": str(var.getDataType()),
                "offset": var.getStackOffset(),
                "address": str(var.getMinAddress()),
            })
        
        # Get basic blocks
        basic_blocks = []
        for block in func.getBody().getBlocks():
            block_start = block.getStart()
            block_end = block.getEnd()
            basic_blocks.append({
                "address": str(block_start),
                "size": block_end.subtract(block_start) + 1,
                "instructions": [str(addr) for addr in block.getAddresses(True)],
            })
        
        functions.append({
            "address": str(entry_point),
            "name": func.getName(),
            "size": body.getNumAddresses(),
            "calling_convention": calling_convention,
            "parameters": parameters,
            "return_type": return_type_str,
            "local_variables": local_vars,
            "basic_blocks": basic_blocks,
        })
    
    return functions

def export_symbols():
    """Export all symbol information"""
    symbols = []
    symbol_table = currentProgram.getSymbolTable()
    
    for symbol in symbol_table.getAllSymbols(True):
        addr = symbol.getAddress()
        if addr is None:
            continue
        
        symbol_type = "Unknown"
        if symbol.isFunction():
            symbol_type = "Function"
        elif symbol.isVariable():
            symbol_type = "Data"
        elif symbol.isLabel():
            symbol_type = "Label"
        
        symbols.append({
            "address": str(addr),
            "name": symbol.getName(),
            "type": symbol_type,
            "namespace": symbol.getParentNamespace().getName() if symbol.getParentNamespace() else None,
        })
    
    return symbols

def export_decompiled_code():
    """Export decompiled C code for functions"""
    decompiled = {}
    func_manager = currentProgram.getFunctionManager()
    decompiler = ghidra.app.decompiler.DecompInterface()
    decompiler.openProgram(currentProgram)
    
    for func in func_manager.getFunctions(True):
        entry_point = func.getEntryPoint()
        result = decompiler.decompileFunction(func, 30, None)
        
        if result.decompileCompleted():
            decompiled[str(entry_point)] = {
                "c_code": result.getDecompiledFunction().getC(),
                "high_function": str(result.getHighFunction()),
            }
    
    return decompiled

def main():
    """Main export function"""
    output_dir = os.getenv("GHIDRA_EXPORT_DIR", "/tmp/ghidra_export")
    os.makedirs(output_dir, exist_ok=True)
    
    print("Exporting functions...")
    functions = export_functions()
    with open(os.path.join(output_dir, "functions.json"), "w") as f:
        json.dump(functions, f, indent=2)
    
    print("Exporting symbols...")
    symbols = export_symbols()
    with open(os.path.join(output_dir, "symbols.json"), "w") as f:
        json.dump(symbols, f, indent=2)
    
    print("Exporting decompiled code...")
    decompiled = export_decompiled_code()
    with open(os.path.join(output_dir, "decompiled.json"), "w") as f:
        json.dump(decompiled, f, indent=2)
    
    print(f"Export complete! Files written to {output_dir}")
    print(f"  - functions.json: {len(functions)} functions")
    print(f"  - symbols.json: {len(symbols)} symbols")
    print(f"  - decompiled.json: {len(decompiled)} decompiled functions")

if __name__ == "__main__":
    main()

