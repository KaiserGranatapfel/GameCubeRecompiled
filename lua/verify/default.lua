-- Default verification suite for recompiled output
-- Runs a series of checks to validate recompilation quality

local function verify(output_path, binary_path)
    local results = {
        passed = 0,
        failed = 0,
        checks = {},
    }

    local function check(name, ok, detail)
        table.insert(results.checks, {
            name = name,
            passed = ok,
            detail = detail or "",
        })
        if ok then
            results.passed = results.passed + 1
            print("[verify] PASS: " .. name)
        else
            results.failed = results.failed + 1
            print("[verify] FAIL: " .. name .. " - " .. (detail or ""))
        end
    end

    -- Check 1: Output file exists and is valid Rust
    if output_path then
        local compiles = gcrecomp.verify.check_compiles(output_path)
        check("Syntax check", compiles, "Balanced braces and function definitions")

        -- Check 2: File size is reasonable
        local size = gcrecomp.verify.file_size(output_path)
        check("File size > 0", size > 0, "Size: " .. size .. " bytes")

        -- Check 3: CRC32 checksum (for reproducibility tracking)
        local crc = gcrecomp.verify.crc32(output_path)
        check("CRC32 computed", crc ~= nil and #crc == 8, "CRC32: " .. (crc or "nil"))

        -- Check 4: SHA256 checksum
        local sha = gcrecomp.verify.sha256(output_path)
        check("SHA256 computed", sha ~= nil and #sha == 64, "SHA256: " .. (sha or "nil"):sub(1, 16) .. "...")
    end

    -- Check 5: Smoke test (if binary available)
    if binary_path then
        local result = gcrecomp.verify.smoke_test(binary_path, 10000)
        check("Smoke test", result.success, result.error or ("Exit code: " .. tostring(result.exit_code)))
    end

    print(string.format("\n[verify] Results: %d passed, %d failed", results.passed, results.failed))
    return results
end

return {
    verify = verify,
}
