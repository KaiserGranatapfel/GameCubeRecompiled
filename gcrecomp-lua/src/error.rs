use thiserror::Error;

#[derive(Error, Debug)]
pub enum LuaBindingError {
    #[error("Lua runtime error: {0}")]
    Runtime(String),

    #[error("Binding error: {0}")]
    Binding(String),

    #[error("Script not found: {0}")]
    ScriptNotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<mlua::Error> for LuaBindingError {
    fn from(err: mlua::Error) -> Self {
        LuaBindingError::Runtime(err.to_string())
    }
}

impl From<LuaBindingError> for mlua::Error {
    fn from(err: LuaBindingError) -> Self {
        mlua::Error::external(err)
    }
}

pub trait IntoAnyhow<T> {
    fn into_anyhow(self) -> anyhow::Result<T>;
}

impl<T> IntoAnyhow<T> for Result<T, mlua::Error> {
    fn into_anyhow(self) -> anyhow::Result<T> {
        self.map_err(|e| anyhow::anyhow!("{}", e))
    }
}
