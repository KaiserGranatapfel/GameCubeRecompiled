use mlua::{Lua, Table};

use crate::convert::{json_to_lua_value, lua_table_to_json};
use crate::error::IntoAnyhow;

/// Config path helper — matches gcrecomp-ui's config location.
fn config_path() -> std::path::PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("gcrecomp");
    path.push("config.json");
    path
}

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let config_table = lua.create_table().into_anyhow()?;

    let load_fn = lua
        .create_function(|lua, ()| {
            let path = config_path();
            if path.exists() {
                let content = std::fs::read_to_string(&path).map_err(mlua::Error::external)?;
                let value: serde_json::Value =
                    serde_json::from_str(&content).map_err(mlua::Error::external)?;
                json_to_lua_value(lua, &value)
            } else {
                // Return sensible defaults
                let defaults = serde_json::json!({
                    "fps_limit": 60,
                    "resolution": [1920, 1080],
                    "vsync": true,
                    "aspect_ratio": "Widescreen",
                    "render_scale": 1.0,
                    "master_volume": 1.0,
                    "music_volume": 1.0,
                    "sfx_volume": 1.0,
                    "audio_backend": "default"
                });
                json_to_lua_value(lua, &defaults)
            }
        })
        .into_anyhow()?;

    let save_fn = lua
        .create_function(|_, tbl: Table| {
            let value = lua_table_to_json(&tbl)?;
            let path = config_path();
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(mlua::Error::external)?;
            }
            let content = serde_json::to_string_pretty(&value).map_err(mlua::Error::external)?;
            std::fs::write(&path, content).map_err(mlua::Error::external)?;
            Ok(())
        })
        .into_anyhow()?;

    config_table.set("load", load_fn).into_anyhow()?;
    config_table.set("save", save_fn).into_anyhow()?;
    gcrecomp.set("config", config_table).into_anyhow()?;
    Ok(())
}
