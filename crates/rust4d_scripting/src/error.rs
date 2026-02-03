//! Error types for the scripting engine

/// Errors that can occur in the scripting engine
#[derive(Debug)]
pub enum ScriptError {
    /// Script file not found
    FileNotFound(String),
    /// IO error reading script file
    IoError(String, std::io::Error),
    /// Lua execution error (syntax error, runtime error)
    LuaError(mlua::Error),
    /// Error in a lifecycle callback
    RuntimeError {
        callback: String,
        error: mlua::Error,
    },
}

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound(path) => write!(f, "Script not found: {}", path),
            Self::IoError(path, e) => write!(f, "Failed to read {}: {}", path, e),
            Self::LuaError(e) => write!(f, "Lua error: {}", e),
            Self::RuntimeError { callback, error } => {
                write!(f, "Error in {}(): {}", callback, error)
            }
        }
    }
}

impl From<mlua::Error> for ScriptError {
    fn from(err: mlua::Error) -> Self {
        ScriptError::LuaError(err)
    }
}

impl std::error::Error for ScriptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(_, e) => Some(e),
            Self::LuaError(e) => Some(e),
            Self::RuntimeError { error, .. } => Some(error),
            _ => None,
        }
    }
}
