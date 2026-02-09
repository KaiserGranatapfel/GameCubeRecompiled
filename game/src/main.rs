// Game entry point
fn main() {
    println!("Game entry point - recompiled code will be integrated here");

    // Initialize Lua scripting engine
    match gcrecomp_lua::engine::LuaEngine::new() {
        Ok(engine) => {
            println!("Lua scripting engine initialized");

            // Load game initialization scripts
            let init_script = std::path::Path::new("lua/game/init.lua");
            if init_script.exists() {
                if let Err(e) = engine.execute_file(init_script) {
                    eprintln!("Failed to load game scripts: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to initialize Lua engine: {}", e);
        }
    }
}
