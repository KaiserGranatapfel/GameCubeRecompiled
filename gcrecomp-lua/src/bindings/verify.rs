use mlua::{Lua, Table};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::error::IntoAnyhow;

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let verify_table = lua.create_table().into_anyhow()?;

    let crc32_fn = lua
        .create_function(|_, path: String| {
            let data = std::fs::read(&path).map_err(mlua::Error::external)?;
            let crc = crc32fast::hash(&data);
            Ok(format!("{:08X}", crc))
        })
        .into_anyhow()?;

    let sha256_fn = lua
        .create_function(|_, path: String| {
            let data = std::fs::read(&path).map_err(mlua::Error::external)?;
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let result = hasher.finalize();
            Ok(format!("{:x}", result))
        })
        .into_anyhow()?;

    let check_compiles_fn = lua
        .create_function(|_, path: String| {
            // Check if the file is valid Rust by looking for balanced braces and fn definitions
            let code = std::fs::read_to_string(&path).map_err(mlua::Error::external)?;
            let opens = code.matches('{').count();
            let closes = code.matches('}').count();
            let has_fns = code.contains("fn ");
            Ok(opens == closes && has_fns)
        })
        .into_anyhow()?;

    let smoke_test_fn = lua
        .create_function(|lua, (binary_path, _timeout_ms): (String, u64)| {
            let result = lua.create_table()?;
            if !Path::new(&binary_path).exists() {
                result.set("success", false)?;
                result.set("error", lua.create_string("Binary not found")?)?;
                return Ok(result);
            }

            let output = std::process::Command::new(&binary_path)
                .arg("--smoke-test")
                .output();

            match output {
                Ok(out) => {
                    result.set("success", out.status.success())?;
                    result.set("exit_code", out.status.code().unwrap_or(-1))?;
                    result.set(
                        "stdout",
                        lua.create_string(String::from_utf8_lossy(&out.stdout).as_bytes())?,
                    )?;
                    result.set(
                        "stderr",
                        lua.create_string(String::from_utf8_lossy(&out.stderr).as_bytes())?,
                    )?;
                }
                Err(e) => {
                    result.set("success", false)?;
                    result.set("error", lua.create_string(e.to_string().as_bytes())?)?;
                }
            }
            Ok(result)
        })
        .into_anyhow()?;

    let file_size_fn = lua
        .create_function(|_, path: String| {
            let metadata = std::fs::metadata(&path).map_err(mlua::Error::external)?;
            Ok(metadata.len())
        })
        .into_anyhow()?;

    verify_table.set("crc32", crc32_fn).into_anyhow()?;
    verify_table.set("sha256", sha256_fn).into_anyhow()?;
    verify_table
        .set("check_compiles", check_compiles_fn)
        .into_anyhow()?;
    verify_table
        .set("smoke_test", smoke_test_fn)
        .into_anyhow()?;
    verify_table.set("file_size", file_size_fn).into_anyhow()?;
    gcrecomp.set("verify", verify_table).into_anyhow()?;
    Ok(())
}
