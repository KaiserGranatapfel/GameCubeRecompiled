-- Audio Settings screen definition
local config = gcrecomp.config.load()

gcrecomp.ui.register_screen("audio_settings", {
    title = "Audio Settings",
    widgets = {
        { type = "slider", id = "master_vol", label = "Master Volume",
          min = 0, max = 100, value = (config.master_volume or 1.0) * 100,
          on_change = "change_master_volume" },
        { type = "slider", id = "music_vol", label = "Music Volume",
          min = 0, max = 100, value = (config.music_volume or 1.0) * 100,
          on_change = "change_music_volume" },
        { type = "slider", id = "sfx_vol", label = "SFX Volume",
          min = 0, max = 100, value = (config.sfx_volume or 1.0) * 100,
          on_change = "change_sfx_volume" },
        { type = "spacer", id = "sp1", style = { height = 10 } },
        { type = "dropdown", id = "audio_backend", label = "Audio Backend",
          options = { "default", "wasapi", "coreaudio", "alsa", "pulseaudio" },
          value = config.audio_backend or "default",
          on_change = "change_audio_backend" },
    }
})
