use std::process::ExitCode;

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    NoWork(String),

    #[error("{0}")]
    Contract(String),

    #[error("{0}")]
    Conflict(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(match self {
            Self::NoWork(_) => 1,
            Self::Contract(_) => 3,
            Self::Conflict(_) => 4,
            Self::Io(_) | Self::Internal(_) => 5,
        })
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::NoWork(_) => "no_work",
            Self::Contract(_) => "contract",
            Self::Conflict(_) => "conflict",
            Self::Io(_) => "io",
            Self::Internal(_) => "internal",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload<'a> {
    pub error: ErrorBody<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorBody<'a> {
    pub kind: &'a str,
    pub message: String,
}

impl<'a> From<&'a AppError> for ErrorPayload<'a> {
    fn from(error: &'a AppError) -> Self {
        Self {
            error: ErrorBody {
                kind: error.kind(),
                message: error.to_string(),
            },
        }
    }
}
