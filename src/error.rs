use std::path::Path;

/// Errors that can leave `main` and are turned into a message plus an exit code.
///
/// Exit codes: `1` for usage/state mistakes (the user asked for something that
/// does not make sense), `2` for I/O and file-format problems.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },

    #[error("{file}:{line}: {message}")]
    Parse {
        file: String,
        line: usize,
        message: String,
    },

    #[error("{0}")]
    Usage(String),

    #[error("{0}")]
    State(String),

    #[error("invalid tournament file: {0}")]
    Json(#[from] serde_json::Error),
}

impl AppError {
    pub fn io(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        AppError::Io {
            path: path.as_ref().display().to_string(),
            source,
        }
    }

    pub fn parse(file: impl AsRef<Path>, line: usize, message: impl Into<String>) -> Self {
        AppError::Parse {
            file: file.as_ref().display().to_string(),
            line,
            message: message.into(),
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        AppError::Usage(message.into())
    }

    pub fn state(message: impl Into<String>) -> Self {
        AppError::State(message.into())
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            AppError::Usage(_) | AppError::State(_) => 1,
            AppError::Io { .. } | AppError::Parse { .. } | AppError::Json(_) => 2,
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
