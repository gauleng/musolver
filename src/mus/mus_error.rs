use thiserror::Error;

#[derive(Debug, Error)]
pub enum MusError {
    #[error("Carácter no válido: {0}")]
    CaracterNoValido(char),
}
