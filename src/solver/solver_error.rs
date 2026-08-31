use thiserror::Error;

use crate::{
    mus::{Accion, MusError},
    solver::CursorMove,
};

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

    #[error("Invalid cursor move in the current node: {0}")]
    InvalidCursorMove(CursorMove),

    #[error("Invalid hand index {0}: the game has {1} hands")]
    InvalidHandIndex(usize, usize),

    #[error("This game type needs {0} hands per player")]
    WrongHandCount(usize),

    #[error("Mus error.")]
    Mus(#[from] MusError),
}
