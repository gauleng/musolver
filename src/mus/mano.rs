use super::mus_error::MusError;
use super::Carta;

#[derive(Debug)]
pub struct Mano(Vec<Carta>);

impl Mano {
    // Cards in hand are always sorted by value.
    pub fn new(mut cartas: Vec<Carta>) -> Self {
        cartas.sort();
        Mano(cartas)
    }

    pub fn codigo(&self) -> usize {
        (self.0[0].valor() as usize) << 24
            | (self.0[1].valor() as usize) << 16
            | (self.0[2].valor() as usize) << 8
            | self.0[3].valor() as usize
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

impl std::fmt::Display for Mano {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let a: String = self.0.iter().map(|c| char::from(c)).collect();
        write!(f, "{a}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let m = Mano::new(vec![Carta::Caballo, Carta::Tres, Carta::Dos, Carta::Siete]);
        assert_eq!(format!("{m}"), "27C3");
    }
}
