//! Error shape returned to the webview.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

impl From<hive_core::HiveError> for CommandError {
    fn from(err: hive_core::HiveError) -> Self {
        Self {
            code: err.code().to_string(),
            message: err.to_string(),
        }
    }
}

impl From<String> for CommandError {
    fn from(message: String) -> Self {
        Self {
            code: "error".into(),
            message,
        }
    }
}

pub type CmdResult<T> = std::result::Result<T, CommandError>;
