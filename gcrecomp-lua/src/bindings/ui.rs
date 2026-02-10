use mlua::{Lua, Table};
use std::sync::{Arc, LazyLock, Mutex};

use crate::error::IntoAnyhow;

/// A Lua-defined screen widget.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LuaWidget {
    pub id: String,
    #[serde(rename = "type")]
    pub widget_type: String,
    pub text: Option<String>,
    pub label: Option<String>,
    pub value: Option<serde_json::Value>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub options: Option<Vec<String>>,
    pub children: Option<Vec<LuaWidget>>,
    pub on_click: Option<String>,
    pub on_change: Option<String>,
    pub enabled: Option<bool>,
    pub style: Option<LuaWidgetStyle>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LuaWidgetStyle {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub padding: Option<f32>,
    pub spacing: Option<f32>,
    pub font_size: Option<f32>,
    pub color: Option<String>,
    pub background: Option<String>,
}

/// A Lua-defined screen definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LuaScreenDef {
    pub id: String,
    pub title: String,
    pub widgets: Vec<LuaWidget>,
}

/// Global registry of Lua-defined screens.
pub static LUA_SCREENS: LazyLock<Arc<Mutex<Vec<LuaScreenDef>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(Vec::new())));

/// Navigation stack for screen history.
pub static NAV_STACK: LazyLock<Arc<Mutex<Vec<String>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(Vec::new())));

/// Toast messages queue.
pub static TOAST_QUEUE: LazyLock<Arc<Mutex<Vec<(String, u64)>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(Vec::new())));

fn parse_widget(w: &Table) -> mlua::Result<LuaWidget> {
    let widget_type: String = w.get("type")?;
    let id: String = w.get("id").unwrap_or_else(|_| String::new());

    let children = if let Ok(children_table) = w.get::<Table>("children") {
        let mut child_widgets = Vec::new();
        for i in 1..=children_table.raw_len() {
            let child: Table = children_table.get(i)?;
            child_widgets.push(parse_widget(&child)?);
        }
        Some(child_widgets)
    } else {
        None
    };

    let options = if let Ok(opts_table) = w.get::<Table>("options") {
        let mut opts = Vec::new();
        for i in 1..=opts_table.raw_len() {
            let opt: String = opts_table.get(i)?;
            opts.push(opt);
        }
        Some(opts)
    } else {
        None
    };

    let value = if let Ok(v) = w.get::<mlua::Value>("value") {
        match v {
            mlua::Value::Boolean(b) => Some(serde_json::Value::Bool(b)),
            mlua::Value::Integer(i) => Some(serde_json::json!(i)),
            mlua::Value::Number(n) => Some(serde_json::json!(n)),
            mlua::Value::String(s) => Some(serde_json::Value::String(s.to_str()?.to_string())),
            _ => None,
        }
    } else {
        None
    };

    let style = if let Ok(style_table) = w.get::<Table>("style") {
        Some(LuaWidgetStyle {
            width: style_table.get("width").ok(),
            height: style_table.get("height").ok(),
            padding: style_table.get("padding").ok(),
            spacing: style_table.get("spacing").ok(),
            font_size: style_table.get("font_size").ok(),
            color: style_table.get("color").ok(),
            background: style_table.get("background").ok(),
        })
    } else {
        None
    };

    Ok(LuaWidget {
        id,
        widget_type,
        text: w.get("text").ok(),
        label: w.get("label").ok(),
        value,
        min: w.get("min").ok(),
        max: w.get("max").ok(),
        options,
        children,
        on_click: w.get("on_click").ok(),
        on_change: w.get("on_change").ok(),
        enabled: w.get("enabled").ok(),
        style,
    })
}

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let ui_table = lua.create_table().into_anyhow()?;

    let register_screen_fn = lua
        .create_function(|_, (id, def): (String, Table)| {
            let title: String = def.get("title")?;
            let widgets_table: Table = def.get("widgets")?;

            let mut widgets = Vec::new();
            for i in 1..=widgets_table.raw_len() {
                let w: Table = widgets_table.get(i)?;
                widgets.push(parse_widget(&w)?);
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

    let navigate_to_fn = lua
        .create_function(|_, screen_id: String| {
            let mut stack = NAV_STACK
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            stack.push(screen_id);
            Ok(())
        })
        .into_anyhow()?;

    let go_back_fn = lua
        .create_function(|_, ()| {
            let mut stack = NAV_STACK
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            stack.pop();
            Ok(())
        })
        .into_anyhow()?;

    let set_widget_value_fn = lua
        .create_function(
            |_, (screen_id, widget_id, value): (String, String, mlua::Value)| {
                let json_value = match value {
                    mlua::Value::Boolean(b) => serde_json::Value::Bool(b),
                    mlua::Value::Integer(i) => serde_json::json!(i),
                    mlua::Value::Number(n) => serde_json::json!(n),
                    mlua::Value::String(s) => serde_json::Value::String(s.to_str()?.to_string()),
                    _ => serde_json::Value::Null,
                };

                let mut screens = LUA_SCREENS
                    .lock()
                    .map_err(|e| mlua::Error::external(e.to_string()))?;
                if let Some(screen) = screens.iter_mut().find(|s| s.id == screen_id) {
                    if let Some(widget) = screen.widgets.iter_mut().find(|w| w.id == widget_id) {
                        widget.value = Some(json_value);
                    }
                }
                Ok(())
            },
        )
        .into_anyhow()?;

    let get_widget_value_fn = lua
        .create_function(|lua, (screen_id, widget_id): (String, String)| {
            let screens = LUA_SCREENS
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            if let Some(screen) = screens.iter().find(|s| s.id == screen_id) {
                if let Some(widget) = screen.widgets.iter().find(|w| w.id == widget_id) {
                    if let Some(ref val) = widget.value {
                        return match val {
                            serde_json::Value::Bool(b) => Ok(mlua::Value::Boolean(*b)),
                            serde_json::Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    Ok(mlua::Value::Integer(i))
                                } else if let Some(f) = n.as_f64() {
                                    Ok(mlua::Value::Number(f))
                                } else {
                                    Ok(mlua::Value::Nil)
                                }
                            }
                            serde_json::Value::String(s) => {
                                let ls = lua.create_string(s)?;
                                Ok(mlua::Value::String(ls))
                            }
                            _ => Ok(mlua::Value::Nil),
                        };
                    }
                }
            }
            Ok(mlua::Value::Nil)
        })
        .into_anyhow()?;

    let show_toast_fn = lua
        .create_function(|_, (message, duration_ms): (String, u64)| {
            let mut queue = TOAST_QUEUE
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            queue.push((message, duration_ms));
            Ok(())
        })
        .into_anyhow()?;

    ui_table
        .set("register_screen", register_screen_fn)
        .into_anyhow()?;
    ui_table
        .set("list_screens", list_screens_fn)
        .into_anyhow()?;
    ui_table.set("navigate_to", navigate_to_fn).into_anyhow()?;
    ui_table.set("go_back", go_back_fn).into_anyhow()?;
    ui_table
        .set("set_widget_value", set_widget_value_fn)
        .into_anyhow()?;
    ui_table
        .set("get_widget_value", get_widget_value_fn)
        .into_anyhow()?;
    ui_table.set("show_toast", show_toast_fn).into_anyhow()?;
    gcrecomp.set("ui", ui_table).into_anyhow()?;
    Ok(())
}
