/// Lua callback registry — maps "screen.widget.event" keys to Lua function names.
///
/// Since `mlua::RegistryKey` is not `Send`, we store callbacks as function
/// name strings and look them up in Lua globals at invocation time.
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

pub static CALLBACK_REGISTRY: LazyLock<Arc<Mutex<CallbackRegistry>>> =
    LazyLock::new(|| Arc::new(Mutex::new(CallbackRegistry::new())));

#[derive(Default)]
pub struct CallbackRegistry {
    /// Maps callback keys ("screen_id.widget_id.event") to Lua global function names.
    keys: HashMap<String, String>,
}

impl CallbackRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a Lua function name as a callback for a given key.
    pub fn register(&mut self, key: &str, func_name: &str) {
        self.keys.insert(key.to_string(), func_name.to_string());
    }

    /// Invoke a registered callback by key. Looks up the function name in
    /// Lua globals and calls it with the given arguments.
    pub fn invoke(
        &self,
        lua: &mlua::Lua,
        key: &str,
        args: impl mlua::IntoLuaMulti,
    ) -> mlua::Result<Option<mlua::Value>> {
        if let Some(func_name) = self.keys.get(key) {
            let globals = lua.globals();
            if let Ok(func) = globals.get::<mlua::Function>(func_name.as_str()) {
                let result = func.call(args)?;
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    /// Check if a callback is registered for a key.
    pub fn has_callback(&self, key: &str) -> bool {
        self.keys.contains_key(key)
    }

    /// Remove a callback.
    pub fn remove(&mut self, key: &str) {
        self.keys.remove(key);
    }

    /// Clear all callbacks.
    pub fn clear(&mut self) {
        self.keys.clear();
    }
}
