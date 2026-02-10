use mlua::{Lua, Table, UserData, UserDataMethods};
use std::sync::{Arc, Mutex};

use gcrecomp_core::runtime::context::CpuContext;

use crate::error::IntoAnyhow;

pub struct LuaCpuContext {
    pub inner: Arc<Mutex<CpuContext>>,
}

impl UserData for LuaCpuContext {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get_gpr", |_, this, reg: u8| {
            let ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            Ok(ctx.get_register(reg))
        });

        methods.add_method("set_gpr", |_, this, (reg, val): (u8, u32)| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            ctx.set_register(reg, val);
            Ok(())
        });

        methods.add_method("get_fpr", |_, this, reg: u8| {
            let ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            Ok(ctx.get_fpr(reg))
        });

        methods.add_method("set_fpr", |_, this, (reg, val): (u8, f64)| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            ctx.set_fpr(reg, val);
            Ok(())
        });

        methods.add_method("get_pc", |_, this, ()| {
            let ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            Ok(ctx.pc)
        });

        methods.add_method("set_pc", |_, this, val: u32| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            ctx.pc = val;
            Ok(())
        });

        methods.add_method("get_lr", |_, this, ()| {
            let ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            Ok(ctx.lr)
        });

        methods.add_method("set_lr", |_, this, val: u32| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            ctx.lr = val;
            Ok(())
        });

        methods.add_method("get_cr", |_, this, ()| {
            let ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            Ok(ctx.cr)
        });

        methods.add_method("get_cr_field", |_, this, field: u8| {
            let ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            Ok(ctx.get_cr_field(field))
        });

        methods.add_method("set_cr_field", |_, this, (field, val): (u8, u8)| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            ctx.set_cr_field(field, val);
            Ok(())
        });
    }
}

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let cpu_table = lua.create_table().into_anyhow()?;

    let new_fn = lua
        .create_function(|_, ()| {
            Ok(LuaCpuContext {
                inner: Arc::new(Mutex::new(CpuContext::new())),
            })
        })
        .into_anyhow()?;

    cpu_table.set("new", new_fn).into_anyhow()?;
    gcrecomp.set("cpu", cpu_table).into_anyhow()?;
    Ok(())
}
