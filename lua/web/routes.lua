-- Web route handlers for GCRecomp dashboard
-- These handlers are called by the Rust web server to process API requests

local routes = {}

function routes.handle_recompile(params)
    local dol_path = params.dol_path or "uploads/uploaded.dol"
    local output_path = params.output_path or "output/recompiled.rs"

    local ctx = gcrecomp.pipeline.new_context()
    ctx:load_dol(dol_path)
    ctx:analyze()
    ctx:decode()
    ctx:build_cfg()
    ctx:analyze_data_flow()
    ctx:infer_types()
    ctx:generate_code()
    ctx:validate()
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

return routes
