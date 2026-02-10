pub mod callbacks;
pub mod config;
pub mod cpu;
pub mod memory;
pub mod optimize;
pub mod pipeline;
pub mod runtime;
pub mod ui;
pub mod verify;

use mlua::Lua;

use crate::error::IntoAnyhow;

pub fn register_all(lua: &Lua) -> anyhow::Result<()> {
    let gcrecomp = lua.create_table().into_anyhow()?;

    config::register(lua, &gcrecomp)?;
    pipeline::register(lua, &gcrecomp)?;
    cpu::register(lua, &gcrecomp)?;
    memory::register(lua, &gcrecomp)?;
    ui::register(lua, &gcrecomp)?;
    verify::register(lua, &gcrecomp)?;
    optimize::register(lua, &gcrecomp)?;
    runtime::register(lua, &gcrecomp)?;

    lua.globals().set("gcrecomp", gcrecomp).into_anyhow()?;
    Ok(())
}
