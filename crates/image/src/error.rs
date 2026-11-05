//! Error type of the crate.

use std::fmt;

#[derive(Debug)]
pub enum ImageError {
    /// The bytes do not start with a known image signature.
    UnknownFormat,
    /// A valid file that uses a feature we do not read or write.
    Unsupported(String),
    /// Corrupt or truncated data.
    Decode(String),
    /// The encoder refused the data.
    Encode(String),
    /// The declared size exceeds the configured limits.
    TooLarge { width: u64, height: u64 },
    /// Bad arguments (empty layer name, sample count not matching the size, ...).
    InvalidInput(String),
    /// File system error (cache loads).
    Io(std::io::Error),
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::UnknownFormat => write!(f, "unknown image format"),
            ImageError::Unsupported(s) => write!(f, "unsupported: {s}"),
            ImageError::Decode(s) => write!(f, "decode error: {s}"),
            ImageError::Encode(s) => write!(f, "encode error: {s}"),
            ImageError::TooLarge { width, height } => {
                write!(f, "image too large: {width}x{height}")
            }
            ImageError::InvalidInput(s) => write!(f, "invalid input: {s}"),
            ImageError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for ImageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ImageError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ImageError {
    fn from(e: std::io::Error) -> Self {
        ImageError::Io(e)
    }
}
