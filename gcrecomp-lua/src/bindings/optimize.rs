use mlua::{Lua, Table};
use std::path::Path;

use crate::error::IntoAnyhow;

pub fn register(lua: &Lua, gcrecomp: &Table) -> anyhow::Result<()> {
    let optimize_table = lua.create_table().into_anyhow()?;

    let dce_fn = lua
        .create_function(|_, path: String| {
            let code = std::fs::read_to_string(&path).map_err(mlua::Error::external)?;
            let original_size = code.len();

            // Simple dead code elimination: remove empty stub functions
            let lines: Vec<&str> = code.lines().collect();
            let mut optimized = String::with_capacity(code.len());
            let mut removed = 0usize;
            let mut i = 0;

            while i < lines.len() {
                let line = lines[i];
                // Detect stub functions (Ok(None) body only)
                if line.trim_start().starts_with("pub fn ")
                    && i + 2 < lines.len()
                    && lines[i + 1].trim() == "Ok(None)"
                    && lines[i + 2].trim() == "}"
                {
                    removed += 1;
                    i += 3; // Skip the stub function
                    // Also skip trailing newline
                    if i < lines.len() && lines[i].is_empty() {
                        i += 1;
                    }
                    continue;
                }
                optimized.push_str(line);
                optimized.push('\n');
                i += 1;
            }

            std::fs::write(&path, &optimized).map_err(mlua::Error::external)?;

            Ok((original_size, optimized.len(), removed))
        })
        .into_anyhow()?;

    let strip_comments_fn = lua
        .create_function(|_, path: String| {
            let code = std::fs::read_to_string(&path).map_err(mlua::Error::external)?;
            let original_size = code.len();

            let optimized: String = code
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !trimmed.starts_with("//") || trimmed.starts_with("//!")
                })
                .collect::<Vec<_>>()
                .join("\n");

            std::fs::write(&path, &optimized).map_err(mlua::Error::external)?;
            Ok((original_size, optimized.len()))
        })
        .into_anyhow()?;

    let size_report_fn = lua
        .create_function(|lua, path: String| {
            let table = lua.create_table()?;

            if Path::new(&path).exists() {
                let metadata =
                    std::fs::metadata(&path).map_err(mlua::Error::external)?;
                let size = metadata.len();
                table.set("size_bytes", size)?;
                table.set("size_kb", size as f64 / 1024.0)?;
                table.set("size_mb", size as f64 / (1024.0 * 1024.0))?;

                // Count functions
                let code = std::fs::read_to_string(&path).map_err(mlua::Error::external)?;
                let fn_count = code.matches("pub fn ").count();
                let line_count = code.lines().count();
                table.set("functions", fn_count)?;
                table.set("lines", line_count)?;
            } else {
                table.set("error", "File not found")?;
            }
            Ok(table)
        })
        .into_anyhow()?;

    optimize_table.set("dce", dce_fn).into_anyhow()?;
    optimize_table
        .set("strip_comments", strip_comments_fn)
        .into_anyhow()?;
    optimize_table
        .set("size_report", size_report_fn)
        .into_anyhow()?;
    gcrecomp.set("optimize", optimize_table).into_anyhow()?;
    Ok(())
}
