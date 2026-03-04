use thiserror::Error;

#[derive(Debug, Error)]
pub enum FlowstateError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("{0}")]
    Database(#[from] rusqlite::Error),

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl FlowstateError {
    pub fn exit_code(&self) -> i32 {
        match self {
            FlowstateError::NotFound(_) => 1,
            FlowstateError::Validation(_) => 2,
            FlowstateError::Conflict(_) => 3,
            FlowstateError::Database(_) | FlowstateError::Other(_) => 1,
        }
    }
}
