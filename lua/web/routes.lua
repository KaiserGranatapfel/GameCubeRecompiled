-- Web route handlers for GCRecomp dashboard
-- ALL business logic lives here; Rust is just HTTP transport.

local templates = require("web.templates")

local routes = {}

--- GET / -> full HTML page
function routes.handle_index()
    local targets = gcrecomp.web.valid_targets()
    local html = templates.index(targets)
    return { status = 200, content_type = "text/html", body = html }
end

--- POST /api/upload
-- params.file_name   (string)  - original filename from header
-- params.game_title  (string)  - game title from header
-- params.target      (string)  - target platform id from header
-- params.body        (string)  - raw binary body (Lua binary string)
-- params.body_len    (integer) - body length in bytes
function routes.handle_upload(params)
    local file_name  = params.file_name or ""
    local game_title = params.game_title or ""
    local target     = params.target or ""
    local body       = params.body
    local body_len   = params.body_len or 0

    -- Validate game title
    game_title = game_title:match("^%s*(.-)%s*$") or "" -- trim
    if game_title == "" then
        game_title = "game"
    end
    if #game_title > 64 then
        game_title = game_title:sub(1, 64)
    end

    -- Validate target
    local targets = gcrecomp.web.valid_targets()
    local valid = false
    local target_names = {}
    for _, t in ipairs(targets) do
        target_names[#target_names + 1] = t.id
        if t.id == target then valid = true end
    end
    if not valid then
        return {
            status = 400,
            body = "Unknown target '" .. target .. "'. Valid targets: " .. table.concat(target_names, ", ")
        }
    end

    -- Validate file extension
    local name_lower = file_name:lower()
    local valid_ext = name_lower:match("%.dol$")
        or name_lower:match("%.zip$")
        or name_lower:match("%.iso$")
        or name_lower:match("%.gcm$")
        or name_lower:match("%.rvz$")
    if file_name ~= "" and not valid_ext then
        return { status = 400, body = "Invalid file type. Accepted: .dol, .zip, .iso, .gcm, .rvz" }
    end

    -- Validate body
    if body == nil or body_len == 0 then
        return { status = 400, body = "No file provided. Please select a file." }
    end

    local max_size = gcrecomp.web.max_upload_size()
    if body_len > max_size then
        return { status = 413, body = "File too large (max 5 GB)." }
    end

    -- Extract DOL based on file type
    local dol_data = body
    if name_lower:match("%.zip$") then
        local extracted, err = gcrecomp.web.extract_dol_from_zip(body)
        if not extracted then
            return { status = 400, body = err or "Failed to extract DOL from zip." }
        end
        dol_data = extracted
    elseif name_lower:match("%.rvz$") then
        -- Combined extraction: converts RVZ→ISO once, extracts both DOL and FST files
        local extracted, err = gcrecomp.web.extract_dol_and_files_from_rvz(body)
        if not extracted then
            return { status = 400, body = err or "Failed to extract DOL from RVZ." }
        end
        dol_data = extracted
    elseif name_lower:match("%.iso$") or name_lower:match("%.gcm$") then
        local extracted, err = gcrecomp.web.extract_dol_from_disc(body)
        if not extracted then
            return { status = 400, body = err or "Failed to extract DOL from disc image." }
        end
        dol_data = extracted

        -- Extract filesystem files for asset embedding (non-fatal)
        local count, fs_err = gcrecomp.web.extract_files_from_disc(body)
        if count then
            print("Extracted " .. count .. " files from disc filesystem")
        elseif fs_err then
            print("Warning: FST extraction failed: " .. fs_err)
        end
    end

    -- Validate DOL magic
    if not gcrecomp.web.validate_dol(dol_data) then
        return {
            status = 400,
            body = "Invalid DOL file. The file does not appear to be a valid GameCube DOL binary."
        }
    end

    -- Save to disk
    gcrecomp.web.save_dol(dol_data, "uploads/uploaded.dol")

    return {
        status = 200,
        body = { status = "started", size = body_len },
        _start_pipeline = true,
        _game_title = game_title,
        _target = target,
    }
end

--- Pipeline recompilation (called in a fresh LuaEngine per pipeline run)
-- params.dol_path    (string)
-- params.output_path (string)
-- params.target      (string)
-- params.game_title  (string)
function routes.handle_recompile(params)
    local dol_path = params.dol_path or "uploads/uploaded.dol"
    local output_path = params.output_path or "output/recompiled.rs"
    local target = params.target or "x86_64-linux"

    gcrecomp.web.update_status("load_dol", "Loading DOL file...")
    local ctx = gcrecomp.pipeline.new_context()
    ctx:load_dol(dol_path)

    gcrecomp.web.update_status("analyze", "Analyzing binary for " .. target .. "...")
    ctx:analyze()

    gcrecomp.web.update_status("decode", "Decoding instructions...")
    ctx:decode()

    gcrecomp.web.update_status("build_cfg", "Building control flow graph...")
    ctx:build_cfg()

    gcrecomp.web.update_status("data_flow", "Analyzing data flow...")
    ctx:analyze_data_flow()

    gcrecomp.web.update_status("type_inference", "Inferring types...")
    ctx:infer_types()

    gcrecomp.web.update_status("codegen", "Generating code for " .. target .. "...")
    ctx:generate_code()

    gcrecomp.web.update_status("validate", "Validating output...")
    ctx:validate()

    gcrecomp.web.update_status("write_output", "Writing output files...")
    ctx:write_output(output_path)

    -- Embed disc assets into game binary
    gcrecomp.web.update_status("embed_assets", "Embedding disc assets...")
    gcrecomp.pipeline.embed_assets()

    -- Compile game executable
    gcrecomp.web.update_status("compile", "Compiling game executable...")
    local game_title = params.game_title or "game"
    local filename, err = gcrecomp.web.compile_game(game_title, target)
    if not filename then
        error("Compilation failed: " .. (err or "unknown error"))
    end

    local stats = ctx:get_stats()
    stats.binary_path = filename
    return stats
end

--- GET /api/config
function routes.handle_config_get()
    return gcrecomp.config.load()
end

--- PUT /api/config
function routes.handle_config_set(config)
    gcrecomp.config.save(config)
    return { status = "saved" }
end

--- GET /api/targets
function routes.handle_list_targets()
    return { targets = gcrecomp.web.valid_targets() }
end

return routes
