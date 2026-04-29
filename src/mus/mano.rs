use std::str::FromStr;

use arrayvec::ArrayVec;

use super::Carta;
use super::mus_error::MusError;

/// Representación de una mano de cartas, no específicamente de mus. Internamente es un vector de
/// Carta.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
pub struct Mano(ArrayVec<Carta, 4>);

impl Mano {
    // Cards in hand are always sorted by value.
    pub fn new(cartas: [Carta; 4]) -> Self {
        let mut m = Mano(ArrayVec::from(cartas));
        m.0.sort_unstable_by(|a, b| b.cmp(a));
        m
    }

    pub fn new_unsorted(cartas: [Carta; 4]) -> Self {
        Mano(ArrayVec::from(cartas))
    }

    // Cards in hand are always sorted by value.
    pub fn from_arrayvec(cartas: ArrayVec<Carta, 4>) -> Self {
        let mut m = Mano(cartas);
        m.0.sort_by(|a, b| b.cmp(a));
        m
    }

    pub fn cartas(&self) -> &[Carta] {
        &self.0
    }

    pub fn insertar(&mut self, carta: Carta) {
        self.0.push(carta);
    }

    pub fn num_figuras(&self) -> u8 {
        self.0.iter().filter(|carta| carta.valor() >= 10).count() as u8
    }
}

impl TryFrom<&str> for Mano {
    type Error = MusError;

    fn try_from(other: &str) -> Result<Self, Self::Error> {
        let mut cartas: ArrayVec<Carta, 4> = ArrayVec::new();
        for c in other.chars() {
            let carta = Carta::try_from(c)?;
            cartas.push(carta);
        }
        Ok(Self::from_arrayvec(cartas))
    }
}

impl FromStr for Mano {
    type Err = MusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Mano::try_from(s)
    }
}

impl std::fmt::Display for Mano {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let a: String = self.0.iter().map(char::from).collect();
        write!(f, "{a}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let m = Mano::new([Carta::Caballo, Carta::Tres, Carta::Dos, Carta::Siete]);
        assert_eq!(format!("{m}"), "3C72");
    }

    #[test]
    fn test_codigo() {
        let m = Mano::new([Carta::As, Carta::As, Carta::As, Carta::Tres]);
        assert_eq!(m.valor_grande(), 201392385);
        assert_eq!(m.valor_chica(), 16843020);
    }
}
