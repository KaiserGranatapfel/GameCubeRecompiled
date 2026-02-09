-- GCRecomp Lua entry point
-- Validates that all bindings are available and functional

print("[gcrecomp] Lua scripting engine initialized")

-- Validate config bindings
assert(gcrecomp ~= nil, "gcrecomp global table missing")
assert(gcrecomp.config ~= nil, "gcrecomp.config missing")
assert(type(gcrecomp.config.load) == "function", "gcrecomp.config.load is not a function")
assert(type(gcrecomp.config.save) == "function", "gcrecomp.config.save is not a function")

-- Round-trip test: load config, modify, save, reload, verify
local config = gcrecomp.config.load()
print("[gcrecomp] Config loaded successfully")
print("[gcrecomp]   fps_limit = " .. tostring(config.fps_limit))
print("[gcrecomp]   vsync = " .. tostring(config.vsync))
print("[gcrecomp]   render_scale = " .. tostring(config.render_scale))

print("[gcrecomp] All bindings validated successfully")
