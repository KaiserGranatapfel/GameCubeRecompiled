#!/usr/bin/env python3
"""
Enhanced Ghidra script to export function and symbol information for GCRecomp
Supports Function ID, BSim, auto-analysis, and advanced symbol extraction
Run this script in Ghidra to export analysis data to JSON
"""

import json
import os
import re
from ghidra.program.model.listing import FunctionManager
from ghidra.program.model.symbol import SymbolTable
from ghidra.program.model.address import AddressSet
from ghidra.program.model.lang import OperandType
from ghidra.app.script import GhidraScript
from ghidra.app.analysis import AnalysisManager

def apply_function_id_databases():
    """Apply Function ID databases to identify known functions"""
    try:
        from ghidra.feature.fid import FidService
        from ghidra.feature.fid.db import FunctionRecord
        
        fid_service = FidService.getFidService()
        if fid_service is None:
            print("Function ID service not available")
            return {}
        
        # Get all FID databases
        databases = fid_service.getAllFidDatabases()
        if not databases:
            print("No Function ID databases found")
            return {}
        
        print(f"Found {len(databases)} Function ID database(s)")
        
        # Apply databases and collect matches
        matches = {}
        func_manager = currentProgram.getFunctionManager()
        
        for func in func_manager.getFunctions(True):
            entry_point = func.getEntryPoint()
            matches_found = []
            
            for db in databases:
                try:
                    # Search for function in database
                    records = fid_service.findRecordsByHash(func.getBody())
                    for record in records:
                        if record:
                            matches_found.append({
                                "name": record.getName(),
                                "library": db.getName(),
                                "confidence": 1.0  # Exact match
                            })
                except:
                    pass
            
            if matches_found:
                matches[str(entry_point)] = matches_found[0]  # Take first match
        
        print(f"Function ID matched {len(matches)} functions")
        return matches
    except ImportError:
        print("Function ID feature not available in this Ghidra version")
        return {}
    except Exception as e:
        print(f"Error applying Function ID: {e}")
        return {}

def run_bsim_analysis():
    """Run BSim fuzzy matching analysis"""
    try:
        from ghidra.feature.bsim import BSimClient
        from ghidra.feature.bsim.query.client import BSimClientFactory
        
        # Check if BSim is available
        # Note: BSim requires external database setup
        print("BSim analysis requires external database - skipping for now")
        return {}
    except ImportError:
        print("BSim feature not available in this Ghidra version")
        return {}
    except Exception as e:
        print(f"Error running BSim analysis: {e}")
        return {}

def run_auto_analyzers():
    """Run all available auto-analyzers"""
    try:
        analysis_manager = currentProgram.getAnalysisManager()
        if analysis_manager is None:
            return
        
        # Get list of available analyzers
        available_analyzers = analysis_manager.getAnalysisOptions()
        
        # Run key analyzers for better symbol resolution
        analyzers_to_run = [
            "DecompilerParameterID",
            "Reference",
            "CreateAddressTables",
            "CreateFunction",
            "RemoveUnusedFunctions",
            "Demangler",
        ]
        
        print("Running auto-analyzers...")
        for analyzer_name in analyzers_to_run:
            try:
                analyzer = available_analyzers.getAnalyzer(analyzer_name)
                if analyzer and not analyzer.isDefault():
                    print(f"  Running {analyzer_name}...")
                    analysis_manager.analyze(analyzer)
            except:
                pass  # Analyzer might not be available
        
        print("Auto-analysis complete")
    except Exception as e:
        print(f"Error running auto-analyzers: {e}")

def detect_namespaces():
    """Detect namespace hierarchy from symbols"""
    namespaces = {}
    symbol_table = currentProgram.getSymbolTable()
    
    for symbol in symbol_table.getAllSymbols(True):
        namespace = symbol.getParentNamespace()
        if namespace and namespace != currentProgram.getGlobalNamespace():
            namespace_name = namespace.getName()
            namespace_path = []
            
            # Build namespace path
            current_ns = namespace
            while current_ns and current_ns != currentProgram.getGlobalNamespace():
                namespace_path.insert(0, current_ns.getName())
                current_ns = current_ns.getParentNamespace()
            
            if namespace_path:
                path_str = "::".join(namespace_path)
                if path_str not in namespaces:
                    namespaces[path_str] = {
                        "path": namespace_path,
                        "symbols": []
                    }
                namespaces[path_str]["symbols"].append({
                    "name": symbol.getName(),
                    "address": str(symbol.getAddress()),
                    "type": "Function" if symbol.isFunction() else "Data"
                })
    
    return namespaces

def detect_sdk_patterns(func_name):
    """Detect common GameCube SDK patterns"""
    patterns = {
        "GX": {"namespace": "graphics", "module": "gx"},
        "VI": {"namespace": "graphics", "module": "vi"},
        "DSP": {"namespace": "audio", "module": "dsp"},
        "AI": {"namespace": "audio", "module": "ai"},
        "OS": {"namespace": "system", "module": "os"},
        "EXI": {"namespace": "system", "module": "exi"},
        "SI": {"namespace": "system", "module": "si"},
        "DVD": {"namespace": "system", "module": "dvd"},
        "CARD": {"namespace": "system", "module": "card"},
    }
    
    for prefix, info in patterns.items():
        if func_name.startswith(prefix):
            return info
    
    return None

def export_functions():
    """Export all function information with enhanced metadata"""
    functions = []
    func_manager = currentProgram.getFunctionManager()
    
    # Get Function ID matches
    fid_matches = apply_function_id_databases()
    
    # Run auto-analyzers for better analysis
    run_auto_analyzers()
    
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
        
        # Get parameters with enhanced type information
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
        
        # Get function name and check for Function ID match
        func_name = func.getName()
        symbol_source = "AutoAnalysis"
        confidence = 0.5
        
        # Check Function ID match
        entry_str = str(entry_point)
        if entry_str in fid_matches:
            match = fid_matches[entry_str]
            func_name = match.get("name", func_name)
            symbol_source = "FunctionId"
            confidence = match.get("confidence", 1.0)
        
        # Detect SDK patterns
        sdk_info = detect_sdk_patterns(func_name)
        namespace_path = []
        module_path = None
        if sdk_info:
            namespace_path = [sdk_info["namespace"]]
            module_path = sdk_info["module"]
        
        # Get namespace from symbol
        namespace = func.getParentNamespace()
        if namespace and namespace != currentProgram.getGlobalNamespace():
            ns_path = []
            current_ns = namespace
            while current_ns and current_ns != currentProgram.getGlobalNamespace():
                ns_path.insert(0, current_ns.getName())
                current_ns = current_ns.getParentNamespace()
            if ns_path:
                namespace_path = ns_path
        
        functions.append({
            "address": entry_str,
            "name": func_name,
            "size": body.getNumAddresses(),
            "calling_convention": calling_convention,
            "parameters": parameters,
            "return_type": return_type_str,
            "local_variables": local_vars,
            "basic_blocks": basic_blocks,
            "symbol_source": symbol_source,
            "confidence": confidence,
            "namespace_path": namespace_path,
            "module_path": module_path,
        })
    
    return functions

def export_symbols():
    """Export all symbol information with enhanced metadata"""
    symbols = []
    symbol_table = currentProgram.getSymbolTable()
    
    # Get Function ID matches for cross-reference
    fid_matches = apply_function_id_databases()
    
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
        
        # Get namespace path
        namespace = symbol.getParentNamespace()
        namespace_path = []
        if namespace and namespace != currentProgram.getGlobalNamespace():
            current_ns = namespace
            while current_ns and current_ns != currentProgram.getGlobalNamespace():
                namespace_path.insert(0, current_ns.getName())
                current_ns = current_ns.getParentNamespace()
        
        # Determine symbol source and confidence
        symbol_source = "AutoAnalysis"
        confidence = 0.5
        
        # Check if this symbol was matched by Function ID
        addr_str = str(addr)
        if addr_str in fid_matches:
            symbol_source = "FunctionId"
            confidence = 1.0
        
        # Detect SDK patterns
        sdk_info = detect_sdk_patterns(symbol.getName())
        module_path = None
        if sdk_info:
            if not namespace_path:
                namespace_path = [sdk_info["namespace"]]
            module_path = sdk_info["module"]
        
        symbols.append({
            "address": addr_str,
            "name": symbol.getName(),
            "type": symbol_type,
            "namespace": namespace.getName() if namespace and namespace != currentProgram.getGlobalNamespace() else None,
            "namespace_path": namespace_path,
            "module_path": module_path,
            "symbol_source": symbol_source,
            "confidence": confidence,
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
    """Main export function with enhanced analysis"""
    output_dir = os.getenv("GHIDRA_EXPORT_DIR", "/tmp/ghidra_export")
    os.makedirs(output_dir, exist_ok=True)
    
    print("=" * 60)
    print("Enhanced GCRecomp Ghidra Export")
    print("=" * 60)
    
    # Run auto-analyzers first for better results
    print("\n[1/5] Running auto-analyzers...")
    run_auto_analyzers()
    
    # Apply Function ID databases
    print("\n[2/5] Applying Function ID databases...")
    fid_matches = apply_function_id_databases()
    
    # Detect namespaces
    print("\n[3/5] Detecting namespaces...")
    namespaces = detect_namespaces()
    with open(os.path.join(output_dir, "namespaces.json"), "w") as f:
        json.dump(namespaces, f, indent=2)
    print(f"  Found {len(namespaces)} namespaces")
    
    # Export functions with enhanced metadata
    print("\n[4/5] Exporting functions with enhanced metadata...")
    functions = export_functions()
    with open(os.path.join(output_dir, "functions.json"), "w") as f:
        json.dump(functions, f, indent=2)
    print(f"  Exported {len(functions)} functions")
    
    # Export symbols with enhanced metadata
    print("\n[5/5] Exporting symbols with enhanced metadata...")
    symbols = export_symbols()
    with open(os.path.join(output_dir, "symbols.json"), "w") as f:
        json.dump(symbols, f, indent=2)
    print(f"  Exported {len(symbols)} symbols")
    
    # Export decompiled code
    print("\nExporting decompiled code...")
    decompiled = export_decompiled_code()
    with open(os.path.join(output_dir, "decompiled.json"), "w") as f:
        json.dump(decompiled, f, indent=2)
    print(f"  Exported {len(decompiled)} decompiled functions")
    
    print("\n" + "=" * 60)
    print(f"Export complete! Files written to {output_dir}")
    print("=" * 60)
    print(f"  - functions.json: {len(functions)} functions")
    print(f"  - symbols.json: {len(symbols)} symbols")
    print(f"  - decompiled.json: {len(decompiled)} decompiled functions")
    print(f"  - namespaces.json: {len(namespaces)} namespaces")
    print("=" * 60)

if __name__ == "__main__":
    main()

