use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameError {
    #[error(
        "Invalid CFR method: {0}. Accepted methods are: cfr, cfr-plus, external-sampling and chance-sampling."
    )]
    InvalidCfrMethod(String),
}
