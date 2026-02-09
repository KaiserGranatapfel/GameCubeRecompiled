-- Default optimization pipeline for recompiled output
-- Applies dead code elimination and size reporting

local function optimize(output_path)
    print("[optimize] Starting optimization pipeline")
    print("[optimize] Input: " .. output_path)

    -- Pre-optimization size report
    local before = gcrecomp.optimize.size_report(output_path)
    print(string.format("[optimize] Before: %.1f KB, %d functions, %d lines",
        before.size_kb, before.functions, before.lines))

    -- Dead code elimination
    local orig, after_dce, removed = gcrecomp.optimize.dce(output_path)
    print(string.format("[optimize] DCE: removed %d stub functions (%.1f KB -> %.1f KB)",
        removed, orig / 1024, after_dce / 1024))

    -- Strip comments
    local before_strip, after_strip = gcrecomp.optimize.strip_comments(output_path)
    print(string.format("[optimize] Strip comments: %.1f KB -> %.1f KB",
        before_strip / 1024, after_strip / 1024))

    -- Post-optimization size report
    local final_report = gcrecomp.optimize.size_report(output_path)
    print(string.format("[optimize] After: %.1f KB, %d functions, %d lines",
        final_report.size_kb, final_report.functions, final_report.lines))

    local reduction = 0
    if before.size_bytes > 0 then
        reduction = (1.0 - final_report.size_bytes / before.size_bytes) * 100
    end
    print(string.format("[optimize] Size reduction: %.1f%%", reduction))

    return {
        before = before,
        after = final_report,
        reduction_percent = reduction,
    }
end

return {
    optimize = optimize,
}
