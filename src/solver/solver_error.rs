use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolverError {
    #[error("Invalid strategy path: {1}")]
    InvalidStrategyPath(#[source] std::io::Error, String),

    #[error("Cannot create folders to specified path: {1}")]
    NoCreateFolderPermission(#[source] std::io::Error, String),

    #[error("Cannot parse JSON strategy file.")]
    ParseStrategyJsonError(#[from] serde_json::Error),

    #[error("Cannot parse RKYV strategy file.")]
    ParseStrategyRkyvError(#[from] rkyv::rancor::Error),

    #[error("Cannot parse strategy file.")]
    UnsupportedFileFormat(String),
}
