use mlua::{Lua, Table};
use std::sync::{Arc, Mutex};

use crate::error::IntoAnyhow;

/// A Lua-defined screen widget.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LuaWidget {
    #[serde(rename = "type")]
    pub widget_type: String,
    pub text: Option<String>,
    pub label: Option<String>,
    pub value: Option<serde_json::Value>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub options: Option<Vec<String>>,
}

/// A Lua-defined screen definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LuaScreenDef {
    pub id: String,
    pub title: String,
    pub widgets: Vec<LuaWidget>,
}

/// Global registry of Lua-defined screens.
pub static LUA_SCREENS: std::sync::LazyLock<Arc<Mutex<Vec<LuaScreenDef>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(Vec::new())));

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let ui_table = lua.create_table().into_anyhow()?;

    let register_screen_fn = lua
        .create_function(|_, (id, def): (String, Table)| {
            let title: String = def.get("title")?;
            let widgets_table: Table = def.get("widgets")?;

            let mut widgets = Vec::new();
            for i in 1..=widgets_table.raw_len() {
                let w: Table = widgets_table.get(i)?;
                let widget = LuaWidget {
                    widget_type: w.get("type")?,
                    text: w.get("text").ok(),
                    label: w.get("label").ok(),
                    value: None,
                    min: w.get("min").ok(),
                    max: w.get("max").ok(),
                    options: None,
                };
                widgets.push(widget);
            }

            let screen_def = LuaScreenDef {
                id: id.clone(),
                title,
                widgets,
            };

            let mut screens = LUA_SCREENS
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;

            // Replace existing screen with same id, or add new
            if let Some(existing) = screens.iter_mut().find(|s| s.id == id) {
                *existing = screen_def;
            } else {
                screens.push(screen_def);
            }

            Ok(())
        })
        .into_anyhow()?;

    let list_screens_fn = lua
        .create_function(|lua, ()| {
            let screens = LUA_SCREENS
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            let table = lua.create_table()?;
            for (i, screen) in screens.iter().enumerate() {
                let s = lua.create_table()?;
                s.set("id", screen.id.as_str())?;
                s.set("title", screen.title.as_str())?;
                table.set(i + 1, s)?;
            }
            Ok(table)
        })
        .into_anyhow()?;

    ui_table
        .set("register_screen", register_screen_fn)
        .into_anyhow()?;
    ui_table
        .set("list_screens", list_screens_fn)
        .into_anyhow()?;
    gcrecomp.set("ui", ui_table).into_anyhow()?;
    Ok(())
}
