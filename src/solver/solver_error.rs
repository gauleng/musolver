use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolverError {
    #[error("Invalid strategy path: {1}")]
    InvalidStrategyPath(#[source] std::io::Error, String),

    #[error("Cannot create folders to specified path: {1}")]
    NoCreateFolderPermission(#[source] std::io::Error, String),

    #[error("Cannot parse strategy file.")]
    StrategyParseJsonError(#[from] serde_json::Error),
}
