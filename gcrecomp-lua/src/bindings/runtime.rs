/// Runtime Lua bindings — expose runtime state to Lua scripts.
use mlua::{Lua, Table};

use crate::error::IntoAnyhow;

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let runtime_table = lua.create_table().into_anyhow()?;

    // gcrecomp.runtime.get_fps() → number
    let get_fps_fn = lua
        .create_function(|_, ()| {
            // Placeholder: in actual runtime, read from frame counter
            Ok(60.0f64)
        })
        .into_anyhow()?;

    // gcrecomp.runtime.get_controller_count() → integer
    let get_controller_count_fn = lua
        .create_function(|_, ()| {
            // Placeholder
            Ok(0i64)
        })
        .into_anyhow()?;

    // gcrecomp.runtime.get_controller_name(id) → string
    let get_controller_name_fn = lua
        .create_function(|_, id: i64| Ok(format!("Controller {}", id)))
        .into_anyhow()?;

    // gcrecomp.runtime.get_resolution() → (width, height)
    let get_resolution_fn = lua
        .create_function(|_, ()| Ok((1920i64, 1080i64)))
        .into_anyhow()?;

    // gcrecomp.runtime.is_running() → boolean
    let is_running_fn = lua.create_function(|_, ()| Ok(true)).into_anyhow()?;

    runtime_table.set("get_fps", get_fps_fn).into_anyhow()?;
    runtime_table
        .set("get_controller_count", get_controller_count_fn)
        .into_anyhow()?;
    runtime_table
        .set("get_controller_name", get_controller_name_fn)
        .into_anyhow()?;
    runtime_table
        .set("get_resolution", get_resolution_fn)
        .into_anyhow()?;
    runtime_table
        .set("is_running", is_running_fn)
        .into_anyhow()?;

    gcrecomp.set("runtime", runtime_table).into_anyhow()?;
    Ok(())
}
