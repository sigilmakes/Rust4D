//! Asset error types
//!
//! Provides error handling for asset loading, caching, and hot-reload operations.

use std::fmt;
use std::io;

/// Error type for asset operations
#[derive(Debug)]
pub enum AssetError {
    /// IO error (file not found, permission denied, etc.)
    Io(io::Error),
    /// Parse error (invalid file format, deserialization failure)
    Parse(String),
    /// Asset not found in the cache
    NotFound(String),
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::Io(err) => write!(f, "Asset IO error: {}", err),
            AssetError::Parse(msg) => write!(f, "Asset parse error: {}", msg),
            AssetError::NotFound(path) => write!(f, "Asset not found: {}", path),
        }
    }
}

impl std::error::Error for AssetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AssetError::Io(err) => Some(err),
            AssetError::Parse(_) => None,
            AssetError::NotFound(_) => None,
        }
    }
}

impl From<io::Error> for AssetError {
    fn from(err: io::Error) -> Self {
        AssetError::Io(err)
    }
}

impl From<String> for AssetError {
    fn from(msg: String) -> Self {
        AssetError::Parse(msg)
    }
}

impl From<&str> for AssetError {
    fn from(msg: &str) -> Self {
        AssetError::Parse(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_io_error_display() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file missing");
        let asset_err = AssetError::Io(io_err);
        let msg = format!("{}", asset_err);
        assert!(msg.contains("IO error"));
        assert!(msg.contains("file missing"));
    }

    #[test]
    fn test_parse_error_display() {
        let err = AssetError::Parse("invalid format".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("parse error"));
        assert!(msg.contains("invalid format"));
    }

    #[test]
    fn test_not_found_error_display() {
        let err = AssetError::NotFound("models/cube.ron".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("not found"));
        assert!(msg.contains("models/cube.ron"));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        let asset_err: AssetError = io_err.into();
        match asset_err {
            AssetError::Io(e) => assert_eq!(e.kind(), io::ErrorKind::PermissionDenied),
            _ => panic!("Expected Io variant"),
        }
    }

    #[test]
    fn test_from_string() {
        let asset_err: AssetError = "bad data".to_string().into();
        match asset_err {
            AssetError::Parse(msg) => assert_eq!(msg, "bad data"),
            _ => panic!("Expected Parse variant"),
        }
    }

    #[test]
    fn test_from_str() {
        let asset_err: AssetError = "bad data".into();
        match asset_err {
            AssetError::Parse(msg) => assert_eq!(msg, "bad data"),
            _ => panic!("Expected Parse variant"),
        }
    }

    #[test]
    fn test_error_source() {
        use std::error::Error;

        let io_err = io::Error::new(io::ErrorKind::NotFound, "missing");
        let asset_err = AssetError::Io(io_err);
        assert!(asset_err.source().is_some());

        let parse_err = AssetError::Parse("bad".to_string());
        assert!(parse_err.source().is_none());

        let not_found_err = AssetError::NotFound("path".to_string());
        assert!(not_found_err.source().is_none());
    }

    #[test]
    fn test_debug_format() {
        let err = AssetError::Parse("test error".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Parse"));
        assert!(debug.contains("test error"));
    }
}
