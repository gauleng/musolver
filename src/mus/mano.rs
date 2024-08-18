use super::mus_error::MusError;
use super::Carta;

#[derive(Debug, Clone)]
pub struct Mano(Vec<Carta>);

impl Mano {
    // Cards in hand are always sorted by value.
    pub fn new(cartas: Vec<Carta>) -> Self {
        let mut m = Mano(cartas);
        m.0.sort();
        m
    }

    /// Convierte la mano a un entero de 4 bytes. La primera carta se mapea al primer byte, la
    /// segunda al segundo byte, y así sucesivamente. Este valor permite ordenar las manos en los
    /// lances de grande y chica.
    pub fn codigo(&self) -> usize {
        (self.0[0].valor() as usize) << 24
            | (self.0[1].valor() as usize) << 16
            | (self.0[2].valor() as usize) << 8
            | self.0[3].valor() as usize
    }

    /// Devuelve el número de parejas de la mano. Si son pares devuelve 1, si son medias devuelve 2
    /// y si son duples 3. En caso de que no haya parejas, devuelve 0.
    pub fn num_parejas(&self) -> usize {
        let p1 = self.0[0].valor() == self.0[1].valor();
        let p2 = self.0[1].valor() == self.0[2].valor();
        let p3 = self.0[2].valor() == self.0[3].valor();

        if p1 && p3 {
            return 3;
        }
        if p1 && p2 || p2 && p3 {
            return 2;
        }
        if p1 || p2 || p3 {
            return 1;
        }

        return 0;
    }

    /// Devuelve los puntos de la mano para los lances de punto y juego.
    pub fn puntos(&self) -> usize {
        self.0.iter().fold(0, |acc, c| {
            if c.valor() >= 10 {
                acc + 10
            } else {
                acc + c.valor() as usize
            }
        })
    }

    /// Devuelve el valor del juego de la mano. Se asigna de forma arbitraria el valor de 42 al
    /// juego de 31, 41 al de 32, y los puntos de la mano en cualquier otro caso. Si la mano no
    /// tiene juego, se devuelve None.
    pub fn juego(&self) -> Option<usize> {
        let p = self.puntos();
        match p {
            31 => Some(42),
            32 => Some(41),
            33..=40 => Some(p),
            _ => None,
        }
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

    #[test]
    fn test_codigo() {
        let m = Mano::new(vec![Carta::As, Carta::As, Carta::As, Carta::Tres]);
        assert_eq!(m.codigo(), 16843020);
    }
}
