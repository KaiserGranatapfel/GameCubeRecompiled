use mlua::{Lua, Table};

use crate::error::IntoAnyhow;

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let config_table = lua.create_table().into_anyhow()?;

    let load_fn = lua
        .create_function(|lua, ()| {
            let config = gcrecomp_ui::config::GameConfig::load().map_err(mlua::Error::external)?;
            let value = serde_json::to_value(&config).map_err(mlua::Error::external)?;
            json_to_lua(lua, &value)
        })
        .into_anyhow()?;

    let save_fn = lua
        .create_function(|_, tbl: Table| {
            let value = lua_table_to_json(&tbl)?;
            let config: gcrecomp_ui::config::GameConfig =
                serde_json::from_value(value).map_err(mlua::Error::external)?;
            config.save().map_err(mlua::Error::external)?;
            Ok(())
        })
        .into_anyhow()?;

    config_table.set("load", load_fn).into_anyhow()?;
    config_table.set("save", save_fn).into_anyhow()?;
    gcrecomp.set("config", config_table).into_anyhow()?;
    Ok(())
}

fn json_to_lua(lua: &Lua, value: &serde_json::Value) -> mlua::Result<mlua::Value> {
    match value {
        serde_json::Value::Null => Ok(mlua::Value::Nil),
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
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua(lua, v)?)?;
            }
            Ok(mlua::Value::Table(table))
        }
        serde_json::Value::Object(map) => {
            let table = lua.create_table()?;
            for (k, v) in map {
                table.set(k.as_str(), json_to_lua(lua, v)?)?;
            }
            Ok(mlua::Value::Table(table))
        }
    }
}

fn lua_table_to_json(table: &Table) -> mlua::Result<serde_json::Value> {
    let len = table.raw_len();
    let is_array = len > 0 && {
        let mut is_seq = true;
        for i in 1..=len {
            if table.raw_get::<mlua::Value>(i)?.is_nil() {
                is_seq = false;
                break;
            }
        }
        is_seq
    };

    if is_array {
        let mut arr = Vec::new();
        for i in 1..=len {
            arr.push(lua_value_to_json(table.raw_get::<mlua::Value>(i)?)?);
        }
        Ok(serde_json::Value::Array(arr))
    } else {
        let mut map = serde_json::Map::new();
        for pair in table.clone().pairs::<mlua::Value, mlua::Value>() {
            let (k, v) = pair?;
            let key = match k {
                mlua::Value::String(s) => s.to_str()?.to_string(),
                mlua::Value::Integer(i) => i.to_string(),
                _ => continue,
            };
            map.insert(key, lua_value_to_json(v)?);
        }
        Ok(serde_json::Value::Object(map))
    }
}

fn lua_value_to_json(value: mlua::Value) -> mlua::Result<serde_json::Value> {
    match value {
        mlua::Value::Nil => Ok(serde_json::Value::Null),
        mlua::Value::Boolean(b) => Ok(serde_json::Value::Bool(b)),
        mlua::Value::Integer(i) => Ok(serde_json::json!(i)),
        mlua::Value::Number(f) => Ok(serde_json::json!(f)),
        mlua::Value::String(s) => Ok(serde_json::Value::String(s.to_str()?.to_string())),
        mlua::Value::Table(t) => lua_table_to_json(&t),
        _ => Ok(serde_json::Value::Null),
    }
}
