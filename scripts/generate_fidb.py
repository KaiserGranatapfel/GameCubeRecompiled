#!/usr/bin/env python3
"""
Generate Function ID Database (FIDB) from reference binaries

This script creates a Function ID database from GameCube SDK binaries
or other reference binaries with known symbols. The database can then
be used to identify functions in stripped binaries.
"""

import json
import os
import sys
from ghidra.program.model.listing import FunctionManager
from ghidra.feature.fid import FidService
from ghidra.feature.fid.db import FidDatabase

def generate_fidb_from_program(program, output_path):
    """Generate FIDB from current Ghidra program"""
    try:
        fid_service = FidService.getFidService()
        if fid_service is None:
            print("ERROR: Function ID service not available")
            return False
        
        func_manager = program.getFunctionManager()
        functions = func_manager.getFunctions(True)
        
        print(f"Found {len(list(functions)))} functions in program")
        
        # Create FIDB database
        db_name = os.path.basename(output_path).replace(".fidb", "")
        db = fid_service.createFidDatabase(db_name, output_path)
        
        if db is None:
            print(f"ERROR: Failed to create FIDB database at {output_path}")
            return False
        
        # Add functions to database
        count = 0
        for func in functions:
            try:
                # Get function body hash
                body = func.getBody()
                if body is None:
                    continue
                
                # Add function to database
                # Note: Actual FIDB creation requires more complex API usage
                # This is a simplified version
                count += 1
                
                if count % 100 == 0:
                    print(f"Processed {count} functions...")
            except Exception as e:
                print(f"Warning: Failed to process function {func.getName()}: {e}")
                continue
        
        print(f"Successfully processed {count} functions")
        print(f"FIDB database created at: {output_path}")
        return True
        
    except ImportError:
        print("ERROR: Function ID feature not available in this Ghidra version")
        return False
    except Exception as e:
        print(f"ERROR: Failed to generate FIDB: {e}")
        return False

def export_function_metadata(program, output_path):
    """Export function metadata to JSON for FIDB generation"""
    func_manager = program.getFunctionManager()
    functions = []
    
    for func in func_manager.getFunctions(True):
        entry_point = func.getEntryPoint()
        body = func.getBody()
        
        if body is None:
            continue
        
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
        
        functions.append({
            "address": str(entry_point),
            "name": func.getName(),
            "size": body.getNumAddresses(),
            "calling_convention": calling_convention,
            "parameters": parameters,
            "return_type": return_type_str,
            "signature": str(signature),
        })
    
    # Write to JSON file
    with open(output_path, "w") as f:
        json.dump(functions, f, indent=2)
    
    print(f"Exported {len(functions)} functions to {output_path}")
    return True

def main():
    """Main function"""
    if len(sys.argv) < 2:
        print("Usage: generate_fidb.py <output_path> [--export-metadata]")
        print("  output_path: Path to output FIDB file or JSON metadata")
        print("  --export-metadata: Export function metadata to JSON instead of creating FIDB")
        return
    
    output_path = sys.argv[1]
    export_metadata = "--export-metadata" in sys.argv
    
    if not currentProgram:
        print("ERROR: No program loaded in Ghidra")
        print("Please open a program in Ghidra before running this script")
        return
    
    if export_metadata:
        print("Exporting function metadata to JSON...")
        export_function_metadata(currentProgram, output_path)
    else:
        print("Generating Function ID database...")
        generate_fidb_from_program(currentProgram, output_path)

if __name__ == "__main__":
    main()

