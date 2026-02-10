-- Controller Configuration screen (Cemu-style)
gcrecomp.ui.register_screen("controller_config", {
    title = "Controller Configuration",
    widgets = {
        -- Controller selector tabs
        { type = "row", id = "controller_tabs", style = { spacing = 5 },
          children = {
              { type = "button", id = "ctrl_1", text = "Controller 1", on_click = "select_ctrl_1" },
              { type = "button", id = "ctrl_2", text = "Controller 2", on_click = "select_ctrl_2" },
              { type = "button", id = "ctrl_3", text = "Controller 3", on_click = "select_ctrl_3" },
              { type = "button", id = "ctrl_4", text = "Controller 4", on_click = "select_ctrl_4" },
          }
        },
        { type = "spacer", id = "sp0", style = { height = 10 } },
        { type = "dropdown", id = "input_device", label = "Input Device",
          options = { "Auto-detect", "Keyboard", "Xbox Controller", "PlayStation Controller", "Switch Pro" },
          value = "Auto-detect", on_change = "change_input_device" },
        { type = "spacer", id = "sp1", style = { height = 10 } },
        { type = "label", id = "mapping_header", text = "Button Mapping",
          style = { font_size = 20 } },
        -- Button mapping rows
        { type = "row", id = "map_a", children = {
            { type = "label", id = "lbl_a", text = "A Button", style = { width = 120 } },
            { type = "button", id = "btn_map_a", text = "Click to map", on_click = "remap_a" },
        }},
        { type = "row", id = "map_b", children = {
            { type = "label", id = "lbl_b", text = "B Button", style = { width = 120 } },
            { type = "button", id = "btn_map_b", text = "Click to map", on_click = "remap_b" },
        }},
        { type = "row", id = "map_x", children = {
            { type = "label", id = "lbl_x", text = "X Button", style = { width = 120 } },
            { type = "button", id = "btn_map_x", text = "Click to map", on_click = "remap_x" },
        }},
        { type = "row", id = "map_y", children = {
            { type = "label", id = "lbl_y", text = "Y Button", style = { width = 120 } },
            { type = "button", id = "btn_map_y", text = "Click to map", on_click = "remap_y" },
        }},
        { type = "row", id = "map_start", children = {
            { type = "label", id = "lbl_start", text = "Start", style = { width = 120 } },
            { type = "button", id = "btn_map_start", text = "Click to map", on_click = "remap_start" },
        }},
        { type = "row", id = "map_l", children = {
            { type = "label", id = "lbl_l", text = "L Trigger", style = { width = 120 } },
            { type = "button", id = "btn_map_l", text = "Click to map", on_click = "remap_l" },
        }},
        { type = "row", id = "map_r", children = {
            { type = "label", id = "lbl_r", text = "R Trigger", style = { width = 120 } },
            { type = "button", id = "btn_map_r", text = "Click to map", on_click = "remap_r" },
        }},
        { type = "row", id = "map_z", children = {
            { type = "label", id = "lbl_z", text = "Z Button", style = { width = 120 } },
            { type = "button", id = "btn_map_z", text = "Click to map", on_click = "remap_z" },
        }},
        { type = "spacer", id = "sp2", style = { height = 10 } },
        { type = "label", id = "stick_header", text = "Stick Settings",
          style = { font_size = 20 } },
        { type = "slider", id = "dead_zone_left", label = "Left Stick Dead Zone",
          min = 0, max = 50, value = 15, on_change = "change_deadzone_left" },
        { type = "slider", id = "dead_zone_right", label = "Right Stick Dead Zone",
          min = 0, max = 50, value = 15, on_change = "change_deadzone_right" },
        { type = "slider", id = "sensitivity_left", label = "Left Stick Sensitivity",
          min = 50, max = 200, value = 100, on_change = "change_sensitivity_left" },
        { type = "slider", id = "sensitivity_right", label = "Right Stick Sensitivity",
          min = 50, max = 200, value = 100, on_change = "change_sensitivity_right" },
        { type = "spacer", id = "sp3", style = { height = 5 } },
        { type = "checkbox", id = "vibration", label = "Vibration / Rumble",
          value = true, on_change = "toggle_vibration" },
        { type = "spacer", id = "sp4", style = { height = 10 } },
        { type = "row", id = "profile_buttons", style = { spacing = 10 },
          children = {
              { type = "button", id = "save_profile", text = "Save Profile", on_click = "save_profile" },
              { type = "button", id = "load_profile", text = "Load Profile", on_click = "load_profile" },
              { type = "button", id = "reset_profile", text = "Reset to Default", on_click = "reset_profile" },
          }
        },
    }
})
