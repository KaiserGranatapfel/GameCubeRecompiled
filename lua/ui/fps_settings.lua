-- FPS Settings screen definition
local config = gcrecomp.config.load()

gcrecomp.ui.register_screen("fps_settings", {
    title = "FPS Settings",
    widgets = {
        { type = "label", id = "fps_label",
          text = "Current FPS Limit: " .. tostring(config.fps_limit or "Unlimited") },
        { type = "spacer", id = "sp1", style = { height = 10 } },
        { type = "button", id = "fps_30", text = "30 FPS",
          on_click = "set_fps_30", style = { width = 200 } },
        { type = "button", id = "fps_60", text = "60 FPS",
          on_click = "set_fps_60", style = { width = 200 } },
        { type = "button", id = "fps_120", text = "120 FPS",
          on_click = "set_fps_120", style = { width = 200 } },
        { type = "button", id = "fps_unlimited", text = "Unlimited",
          on_click = "set_fps_unlimited", style = { width = 200 } },
        { type = "spacer", id = "sp2", style = { height = 10 } },
        { type = "checkbox", id = "vsync_toggle", label = "VSync",
          value = config.vsync or true, on_change = "toggle_vsync" },
    }
})
