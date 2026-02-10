-- Graphics Settings screen definition
local config = gcrecomp.config.load()

gcrecomp.ui.register_screen("graphics_settings", {
    title = "Graphics Settings",
    widgets = {
        { type = "dropdown", id = "resolution", label = "Resolution",
          options = { "640x480", "1280x720", "1920x1080", "2560x1440", "3840x2160" },
          value = tostring(config.resolution[1] or 1920) .. "x" .. tostring(config.resolution[2] or 1080),
          on_change = "change_resolution" },
        { type = "spacer", id = "sp1", style = { height = 5 } },
        { type = "slider", id = "render_scale", label = "Render Scale",
          min = 0.5, max = 4.0, value = config.render_scale or 1.0,
          on_change = "change_render_scale" },
        { type = "spacer", id = "sp2", style = { height = 5 } },
        { type = "dropdown", id = "aspect_ratio", label = "Aspect Ratio",
          options = { "Original (4:3)", "Widescreen (16:9)", "Ultra-Wide (21:9)" },
          value = "Widescreen (16:9)", on_change = "change_aspect_ratio" },
        { type = "spacer", id = "sp3", style = { height = 5 } },
        { type = "checkbox", id = "aa_toggle", label = "Anti-Aliasing",
          value = false, on_change = "toggle_aa" },
        { type = "dropdown", id = "tex_filter", label = "Texture Filtering",
          options = { "Nearest", "Bilinear", "Trilinear", "Anisotropic 4x", "Anisotropic 16x" },
          value = "Bilinear", on_change = "change_tex_filter" },
    }
})
