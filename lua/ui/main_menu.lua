-- Main Menu screen definition
gcrecomp.ui.register_screen("main_menu", {
    title = "Game Settings",
    widgets = {
        { type = "button", id = "fps_btn", text = "FPS Settings",
          on_click = "navigate_fps", style = { width = 250 } },
        { type = "button", id = "gfx_btn", text = "Graphics Settings",
          on_click = "navigate_graphics", style = { width = 250 } },
        { type = "button", id = "audio_btn", text = "Audio Settings",
          on_click = "navigate_audio", style = { width = 250 } },
        { type = "button", id = "controller_btn", text = "Controller Configuration",
          on_click = "navigate_controller", style = { width = 250 } },
        { type = "button", id = "game_btn", text = "Game Settings",
          on_click = "navigate_game", style = { width = 250 } },
        { type = "spacer", id = "sp1", style = { height = 20 } },
        { type = "button", id = "close_btn", text = "Close Menu (ESC)",
          on_click = "close_menu", style = { width = 250 } },
    }
})
