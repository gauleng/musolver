mod carta;
pub use carta::*;

mod mus_error;
pub use mus_error::MusError;

mod mano;
pub use mano::*;

mod lance;
pub use lance::*;

enum Accion {
    Paso,
    Envido(u8),
    Quiero,
    Ordago,
}
