use mlua::{Lua, Table, UserData, UserDataMethods};
use std::sync::{Arc, Mutex};

use gcrecomp_core::recompiler::pipeline::{PipelineContext, RecompilationPipeline};

use crate::error::IntoAnyhow;

struct LuaPipelineContext {
    inner: Arc<Mutex<PipelineContext>>,
}

impl UserData for LuaPipelineContext {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("load_dol", |_, this, path: String| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            RecompilationPipeline::stage_load_dol(&mut ctx, &path)
                .map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method("analyze", |_, this, ()| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            RecompilationPipeline::stage_analyze(&mut ctx).map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method("decode", |_, this, ()| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            RecompilationPipeline::stage_decode(&mut ctx).map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method("build_cfg", |_, this, ()| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            RecompilationPipeline::stage_build_cfg(&mut ctx).map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method("analyze_data_flow", |_, this, ()| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            RecompilationPipeline::stage_analyze_data_flow(&mut ctx)
                .map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method("infer_types", |_, this, ()| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            RecompilationPipeline::stage_infer_types(&mut ctx).map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method("generate_code", |_, this, ()| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            RecompilationPipeline::stage_generate_code(&mut ctx).map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method("validate", |_, this, ()| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            RecompilationPipeline::stage_validate(&mut ctx).map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method("write_output", |_, this, path: String| {
            let mut ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            RecompilationPipeline::stage_write_output(&mut ctx, &path)
                .map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method("get_stats", |lua, this, ()| {
            let ctx = this
                .inner
                .lock()
                .map_err(|e| mlua::Error::external(e.to_string()))?;
            let table = lua.create_table()?;
            table.set("total_functions", ctx.stats.total_functions)?;
            table.set("successful_functions", ctx.stats.successful_functions)?;
            table.set("failed_functions", ctx.stats.failed_functions)?;
            table.set("total_instructions", ctx.stats.total_instructions)?;
            Ok(table)
        });
    }
}

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let pipeline_table = lua.create_table().into_anyhow()?;

    let new_context_fn = lua
        .create_function(|_, ()| {
            Ok(LuaPipelineContext {
                inner: Arc::new(Mutex::new(PipelineContext::new())),
            })
        })
        .into_anyhow()?;

    pipeline_table
        .set("new_context", new_context_fn)
        .into_anyhow()?;
    gcrecomp.set("pipeline", pipeline_table).into_anyhow()?;
    Ok(())
}
