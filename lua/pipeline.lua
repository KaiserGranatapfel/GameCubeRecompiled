-- GCRecomp default pipeline orchestration script
-- Users can customize this script to modify the recompilation pipeline

local function run_pipeline(dol_path, output_path)
    print("[pipeline] Starting recompilation pipeline")
    print("[pipeline] Input: " .. dol_path)
    print("[pipeline] Output: " .. output_path)

    local ctx = gcrecomp.pipeline.new_context()

    print("[pipeline] Loading DOL file...")
    ctx:load_dol(dol_path)

    print("[pipeline] Running Ghidra analysis...")
    ctx:analyze()

    print("[pipeline] Decoding instructions...")
    ctx:decode()

    print("[pipeline] Building control flow graph...")
    ctx:build_cfg()

    print("[pipeline] Analyzing data flow...")
    ctx:analyze_data_flow()

    print("[pipeline] Inferring types...")
    ctx:infer_types()

    print("[pipeline] Generating code...")
    ctx:generate_code()

    print("[pipeline] Validating output...")
    ctx:validate()

    print("[pipeline] Writing output...")
    ctx:write_output(output_path)

    local stats = ctx:get_stats()
    print("[pipeline] Recompilation complete!")
    print("[pipeline]   Total functions: " .. stats.total_functions)
    print("[pipeline]   Successful: " .. stats.successful_functions)
    print("[pipeline]   Failed: " .. stats.failed_functions)
    print("[pipeline]   Total instructions: " .. stats.total_instructions)

    return stats
end

return {
    run = run_pipeline,
}
