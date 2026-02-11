use anyhow::Context;
use mlua::Lua;
use std::path::Path;

use crate::bindings;
use crate::error::IntoAnyhow;

pub struct LuaEngine {
    lua: Lua,
}

impl LuaEngine {
    pub fn new() -> anyhow::Result<Self> {
        let lua = Lua::new();

        bindings::register_all(&lua)?;

        Ok(Self { lua })
    }

    pub fn execute_file(&self, path: &Path) -> anyhow::Result<()> {
        let script = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read Lua script: {}", path.display()))?;
        self.lua
            .load(&script)
            .set_name(path.to_string_lossy())
            .exec()
            .into_anyhow()
            .with_context(|| format!("Failed to execute Lua script: {}", path.display()))?;
        Ok(())
    }

    pub fn execute_string(&self, code: &str) -> anyhow::Result<()> {
        self.lua
            .load(code)
            .exec()
            .into_anyhow()
            .context("Failed to execute Lua string")?;
        Ok(())
    }

    pub fn set_package_path(&self, path: &str) -> anyhow::Result<()> {
        let package: mlua::Table = self.lua.globals().get("package").into_anyhow()?;
        package.set("path", path).into_anyhow()?;
        Ok(())
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}
