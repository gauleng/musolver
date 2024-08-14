use std::cmp;

enum Valor {
    Uno = 1,
    Dos = 2,
    Tres = 3,
    Cuatro = 4,
    Cinco = 5,
    Seis = 6,
    Siete = 7,
    Sota = 10,
    Caballo = 11,
    Rey = 12,
}

enum MusError {
    CaracterNoValido,
}

impl Valor {}

impl TryFrom<char> for Valor {
    type Error = MusError;

    fn try_from(other: char) -> Result<Self, Self::Error> {
        match other {
            '1' => Ok(Valor::Uno),
            _ => Err(MusError::CaracterNoValido),
        }
    }
}

struct Mano(Vec<Valor>);

impl Mano {}

enum Accion {
    Paso,
    Envido(u8),
    Quiero,
    Ordago,
}

trait Lance {
    fn mejor_mano(a: &Valor, b: &Valor) -> cmp::Ordering {
        cmp::Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn name() {
        todo!();
    }
}
