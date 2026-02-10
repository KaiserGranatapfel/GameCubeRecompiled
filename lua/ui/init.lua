-- UI Screen Loader
-- Loads all Lua-defined UI screens for the settings menu

print("[gcrecomp] Loading UI screen definitions...")

-- Load individual screen definitions
local ui_dir = "lua/ui/"
local screens = {
    "main_menu",
    "fps_settings",
    "graphics_settings",
    "audio_settings",
    "controller_config",
    "game_settings",
}

for _, name in ipairs(screens) do
    local path = ui_dir .. name .. ".lua"
    local f = io.open(path, "r")
    if f then
        f:close()
        dofile(path)
        print("[gcrecomp]   Loaded screen: " .. name)
    else
        print("[gcrecomp]   Screen file not found: " .. path)
    end
end

print("[gcrecomp] UI screens loaded")
