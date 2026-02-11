-- Web route handlers for GCRecomp dashboard
-- These handlers are called by the Rust web server to process API requests

local routes = {}

function routes.handle_recompile(params)
    local dol_path = params.dol_path or "uploads/uploaded.dol"
    local output_path = params.output_path or "output/recompiled.rs"

    gcrecomp.web.update_status("load_dol", "Loading DOL file...")
    local ctx = gcrecomp.pipeline.new_context()
    ctx:load_dol(dol_path)

    gcrecomp.web.update_status("analyze", "Analyzing binary...")
    ctx:analyze()

    gcrecomp.web.update_status("decode", "Decoding instructions...")
    ctx:decode()

    gcrecomp.web.update_status("build_cfg", "Building control flow graph...")
    ctx:build_cfg()

    gcrecomp.web.update_status("data_flow", "Analyzing data flow...")
    ctx:analyze_data_flow()

    gcrecomp.web.update_status("type_inference", "Inferring types...")
    ctx:infer_types()

    gcrecomp.web.update_status("codegen", "Generating code...")
    ctx:generate_code()

    gcrecomp.web.update_status("validate", "Validating output...")
    ctx:validate()

    gcrecomp.web.update_status("write_output", "Writing output files...")
    ctx:write_output(output_path)

    return ctx:get_stats()
end

function routes.handle_config_get()
    return gcrecomp.config.load()
end

function routes.handle_config_set(config)
    gcrecomp.config.save(config)
    return { status = "saved" }
end

function routes.handle_list_targets()
    return {
        targets = {
            { id = "x86_64-linux", name = "x86_64 Linux" },
            { id = "x86_64-windows", name = "x86_64 Windows" },
            { id = "aarch64-linux", name = "AArch64 Linux" },
            { id = "aarch64-macos", name = "AArch64 macOS" },
        }
    }
end

return routes
