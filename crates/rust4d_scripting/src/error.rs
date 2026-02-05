//! Error types for the scripting system
//!
//! Provides a unified error type for all scripting-related failures,
//! including file I/O, Lua parsing, and runtime errors.

use std::fmt;
use std::path::PathBuf;

/// Errors that can occur during script execution
#[derive(Debug)]
pub enum ScriptError {
    /// Script file was not found at the expected path
    FileNotFound {
        path: PathBuf,
    },
    /// I/O error while reading script files
    IoError {
        path: PathBuf,
        source: std::io::Error,
    },
    /// Lua syntax or compilation error
    LuaError {
        message: String,
        source: Option<mlua::Error>,
    },
    /// Runtime error during script execution
    RuntimeError {
        callback: String,
        message: String,
        source: Option<mlua::Error>,
    },
    /// Error during hot-reload
    ReloadError {
        message: String,
        source: Option<Box<ScriptError>>,
    },
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptError::FileNotFound { path } => {
                write!(f, "Script file not found: {}", path.display())
            }
            ScriptError::IoError { path, source } => {
                write!(f, "I/O error reading {}: {}", path.display(), source)
            }
            ScriptError::LuaError { message, .. } => {
                write!(f, "Lua error: {}", message)
            }
            ScriptError::RuntimeError { callback, message, .. } => {
                write!(f, "Runtime error in '{}': {}", callback, message)
            }
            ScriptError::ReloadError { message, .. } => {
                write!(f, "Reload error: {}", message)
            }
        }
    }
}

impl std::error::Error for ScriptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ScriptError::IoError { source, .. } => Some(source),
            ScriptError::LuaError { source: Some(e), .. } => Some(e),
            ScriptError::RuntimeError { source: Some(e), .. } => Some(e),
            ScriptError::ReloadError { source: Some(e), .. } => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<mlua::Error> for ScriptError {
    fn from(err: mlua::Error) -> Self {
        // Try to categorize the error based on its type
        let message = err.to_string();

        match &err {
            mlua::Error::RuntimeError(_) | mlua::Error::CallbackError { .. } => {
                ScriptError::RuntimeError {
                    callback: "unknown".to_string(),
                    message,
                    source: Some(err),
                }
            }
            _ => ScriptError::LuaError {
                message,
                source: Some(err),
            },
        }
    }
}

impl ScriptError {
    /// Create a runtime error with callback context
    pub fn runtime(callback: impl Into<String>, err: mlua::Error) -> Self {
        ScriptError::RuntimeError {
            callback: callback.into(),
            message: err.to_string(),
            source: Some(err),
        }
    }

    /// Create a file not found error
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        ScriptError::FileNotFound { path: path.into() }
    }

    /// Create an I/O error
    pub fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        ScriptError::IoError {
            path: path.into(),
            source,
        }
    }

    /// Create a reload error
    pub fn reload(message: impl Into<String>, source: Option<ScriptError>) -> Self {
        ScriptError::ReloadError {
            message: message.into(),
            source: source.map(Box::new),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_file_not_found_display() {
        let err = ScriptError::file_not_found("/scripts/main.lua");
        assert!(err.to_string().contains("not found"));
        assert!(err.to_string().contains("main.lua"));
    }

    #[test]
    fn test_io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = ScriptError::io_error("/scripts/main.lua", io_err);
        assert!(err.to_string().contains("I/O error"));
        assert!(err.to_string().contains("access denied"));
    }

    #[test]
    fn test_lua_error_conversion() {
        let lua_err = mlua::Error::SyntaxError {
            message: "unexpected symbol".to_string(),
            incomplete_input: false,
        };
        let err: ScriptError = lua_err.into();
        match err {
            ScriptError::LuaError { message, .. } => {
                assert!(message.contains("unexpected symbol"));
            }
            _ => panic!("Expected LuaError variant"),
        }
    }

    #[test]
    fn test_runtime_error_with_callback() {
        let lua_err = mlua::Error::RuntimeError("attempt to call nil".to_string());
        let err = ScriptError::runtime("on_update", lua_err);
        assert!(err.to_string().contains("on_update"));
        assert!(err.to_string().contains("attempt to call nil"));
    }

    #[test]
    fn test_reload_error() {
        let inner = ScriptError::file_not_found("/scripts/main.lua");
        let err = ScriptError::reload("failed to reload scripts", Some(inner));
        assert!(err.to_string().contains("Reload error"));
    }

    #[test]
    fn test_error_source_chain() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = ScriptError::io_error("/scripts/main.lua", io_err);
        assert!(err.source().is_some());
    }
}
