use mlua::{Lua, Table, Value};

pub fn json_to_lua_value(lua: &Lua, value: &serde_json::Value) -> mlua::Result<Value> {
    match value {
        serde_json::Value::Null => Ok(Value::Nil),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Number(f))
            } else {
                Ok(Value::Nil)
            }
        }
        serde_json::Value::String(s) => {
            let ls = lua.create_string(s)?;
            Ok(Value::String(ls))
        }
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua_value(lua, v)?)?;
            }
            Ok(Value::Table(table))
        }
        serde_json::Value::Object(map) => {
            let table = lua.create_table()?;
            for (k, v) in map {
                table.set(k.as_str(), json_to_lua_value(lua, v)?)?;
            }
            Ok(Value::Table(table))
        }
    }
}

pub fn lua_table_to_json(table: &Table) -> mlua::Result<serde_json::Value> {
    let len = table.raw_len();
    let is_array = len > 0 && {
        let mut is_seq = true;
        for i in 1..=len {
            if table.raw_get::<Value>(i)?.is_nil() {
                is_seq = false;
                break;
            }
        }
        is_seq
    };

    if is_array {
        let mut arr = Vec::new();
        for i in 1..=len {
            arr.push(lua_value_to_json(table.raw_get::<Value>(i)?)?);
        }
        Ok(serde_json::Value::Array(arr))
    } else {
        let mut map = serde_json::Map::new();
        for pair in table.clone().pairs::<Value, Value>() {
            let (k, v) = pair?;
            let key = match k {
                Value::String(s) => s.to_str()?.to_string(),
                Value::Integer(i) => i.to_string(),
                _ => continue,
            };
            map.insert(key, lua_value_to_json(v)?);
        }
        Ok(serde_json::Value::Object(map))
    }
}

pub fn lua_value_to_json(value: Value) -> mlua::Result<serde_json::Value> {
    match value {
        Value::Nil => Ok(serde_json::Value::Null),
        Value::Boolean(b) => Ok(serde_json::Value::Bool(b)),
        Value::Integer(i) => Ok(serde_json::json!(i)),
        Value::Number(f) => Ok(serde_json::json!(f)),
        Value::String(s) => Ok(serde_json::Value::String(s.to_str()?.to_string())),
        Value::Table(t) => lua_table_to_json(&t),
        _ => Ok(serde_json::Value::Null),
    }
}
