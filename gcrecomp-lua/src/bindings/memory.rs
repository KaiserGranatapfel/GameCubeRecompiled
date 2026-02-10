use mlua::{Lua, Table, UserData, UserDataMethods};
use std::sync::{Arc, Mutex};

use gcrecomp_core::runtime::memory::MemoryManager;

use crate::error::IntoAnyhow;

pub struct LuaMemoryManager {
    pub inner: Arc<Mutex<MemoryManager>>,
}

impl UserData for LuaMemoryManager {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("read_u8", |_, this, addr: u32| {
            let mem = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            mem.read_u8(addr).map_err(mlua::Error::external)
        });

        methods.add_method("read_u16", |_, this, addr: u32| {
            let mem = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            mem.read_u16(addr).map_err(mlua::Error::external)
        });

        methods.add_method("read_u32", |_, this, addr: u32| {
            let mem = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            mem.read_u32(addr).map_err(mlua::Error::external)
        });

        methods.add_method("read_u64", |_, this, addr: u32| {
            let mem = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            mem.read_u64(addr).map_err(mlua::Error::external)
        });

        methods.add_method("read_bytes", |_, this, (addr, len): (u32, usize)| {
            let mem = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            let bytes = mem.read_bytes(addr, len).map_err(mlua::Error::external)?;
            Ok(bytes)
        });

        methods.add_method("write_u8", |_, this, (addr, val): (u32, u8)| {
            let mut mem = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            mem.write_u8(addr, val).map_err(mlua::Error::external)
        });

        methods.add_method("write_u16", |_, this, (addr, val): (u32, u16)| {
            let mut mem = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            mem.write_u16(addr, val).map_err(mlua::Error::external)
        });

        methods.add_method("write_u32", |_, this, (addr, val): (u32, u32)| {
            let mut mem = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            mem.write_u32(addr, val).map_err(mlua::Error::external)
        });

        methods.add_method("write_u64", |_, this, (addr, val): (u32, u64)| {
            let mut mem = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            mem.write_u64(addr, val).map_err(mlua::Error::external)
        });

        methods.add_method("write_bytes", |_, this, (addr, data): (u32, Vec<u8>)| {
            let mut mem = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            mem.write_bytes(addr, &data).map_err(mlua::Error::external)
        });
    }
}

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let memory_table = lua.create_table().into_anyhow()?;

    let new_fn = lua
        .create_function(|_, ()| {
            Ok(LuaMemoryManager {
                inner: Arc::new(Mutex::new(MemoryManager::new())),
            })
        })
        .into_anyhow()?;

    memory_table.set("new", new_fn).into_anyhow()?;
    gcrecomp.set("memory", memory_table).into_anyhow()?;
    Ok(())
}
