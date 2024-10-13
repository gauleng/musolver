use thiserror::Error;

#[derive(Debug, Error)]
pub enum SolverError {
    #[error("Invalid strategy path: {1}")]
    InvalidStrategyPath(#[source] std::io::Error, String),

    #[error("Cannot parse strategy file.")]
    StrategyParseError(#[from] serde_json::Error),
}
