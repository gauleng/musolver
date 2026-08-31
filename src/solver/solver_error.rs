use thiserror::Error;

use crate::mus::Accion;

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

    #[error("Selected action not in game abstraction: {0}")]
    ActionNotInAbstraction(Accion),

    #[error("Invalid number of discarded cards: {0}")]
    InvalidDiscardsNumber(usize),
}
