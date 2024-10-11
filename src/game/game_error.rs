use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError {
    #[error("Invalid CFR method: {0}")]
    InvalidCfrMethod(String),
}
