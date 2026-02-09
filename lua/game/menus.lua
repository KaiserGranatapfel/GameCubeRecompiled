-- Default menu definitions for in-game overlay
-- Users can modify this file to customize menus

local menus = {}

-- FPS settings screen
gcrecomp.ui.register_screen("lua_fps", {
    title = "FPS Settings (Lua)",
    widgets = {
        { type = "button", text = "30 FPS" },
        { type = "button", text = "60 FPS" },
        { type = "button", text = "Unlimited" },
    },
})
table.insert(menus, "lua_fps")

-- Quick settings screen
gcrecomp.ui.register_screen("lua_quick", {
    title = "Quick Settings",
    widgets = {
        { type = "button", text = "Toggle VSync" },
        { type = "button", text = "Toggle Widescreen" },
        { type = "button", text = "Reset to Defaults" },
    },
})
table.insert(menus, "lua_quick")

return menus
