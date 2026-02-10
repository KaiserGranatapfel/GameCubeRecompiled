-- Game-specific settings
gcrecomp.ui.register_screen("game_settings", {
    title = "Game Settings",
    widgets = {
        { type = "label", id = "info", text = "Game-specific settings will appear here when a game is loaded." },
        { type = "spacer", id = "sp1", style = { height = 10 } },
        { type = "checkbox", id = "widescreen_hack", label = "Widescreen Hack",
          value = false, on_change = "toggle_widescreen_hack" },
        { type = "checkbox", id = "skip_intro", label = "Skip Intro Videos",
          value = false, on_change = "toggle_skip_intro" },
        { type = "dropdown", id = "language", label = "Language",
          options = { "English", "Japanese", "German", "French", "Spanish", "Italian" },
          value = "English", on_change = "change_language" },
    }
})
