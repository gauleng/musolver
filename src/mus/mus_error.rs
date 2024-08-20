use thiserror::Error;

#[derive(Debug, Error)]
pub enum MusError {
    #[error("Carácter no válido: {0}")]
    CaracterNoValido(char),

    #[error("Valor de carta no válido: {0}")]
    ValorNoValido(u8),

    #[error("Acción no válida")]
    AccionNoValida,
}
