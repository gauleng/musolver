use std::str::FromStr;

use super::Carta;
use super::mus_error::MusError;

/// Representación de una mano de cartas, no específicamente de mus. Internamente es un vector de
/// Carta.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Default)]
pub struct Mano(Vec<Carta>);

impl Mano {
    // Cards in hand are always sorted by value.
    pub fn new(cartas: Vec<Carta>) -> Self {
        let mut m = Mano(cartas);
        m.0.sort_by(|a, b| b.cmp(a));
        m
    }

    pub fn cartas(&self) -> &Vec<Carta> {
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
        let mut cartas: Vec<Carta> = Vec::new();
        for c in other.chars() {
            let carta = Carta::try_from(c)?;
            cartas.push(carta);
        }
        Ok(Self::new(cartas))
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
        let m = Mano::new(vec![Carta::Caballo, Carta::Tres, Carta::Dos, Carta::Siete]);
        assert_eq!(format!("{m}"), "3C72");
    }

    #[test]
    fn test_codigo() {
        let m = Mano::new(vec![Carta::As, Carta::As, Carta::As, Carta::Tres]);
        assert_eq!(m.valor_grande(), 201392385);
        assert_eq!(m.valor_chica(), 16843020);
    }
}
