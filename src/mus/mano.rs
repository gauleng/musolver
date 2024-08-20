use super::mus_error::MusError;
use super::Carta;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Mano(Vec<Carta>);

#[derive(PartialOrd, Ord, PartialEq, Eq)]
pub enum Juego {
    Resto(u8),
    Treintaydos,
    Treintayuna,
}

#[derive(PartialOrd, Ord, PartialEq, Eq)]
pub enum Pares {
    Pareja(u16),
    Medias(u16),
    Duples(u16),
}

impl Mano {
    // Cards in hand are always sorted by value.
    pub fn new(cartas: Vec<Carta>) -> Self {
        let mut m = Mano(cartas);
        m.0.sort();
        m
    }

    /// Convierte la mano a un entero de 4 bytes. La cuarta carta se mapea al primer byte, la
    /// tercera al segundo byte, y así sucesivamente. Este valor permite ordenar las manos en el
    /// lance de grande.
    pub fn valor_grande(&self) -> usize {
        (self.0[3].valor() as usize) << 24
            | (self.0[2].valor() as usize) << 16
            | (self.0[1].valor() as usize) << 8
            | self.0[0].valor() as usize
    }

    /// Convierte la mano a un entero de 4 bytes. La primera carta se mapea al primer byte, la
    /// segunda al segundo byte, y así sucesivamente. Este valor permite ordenar las manos en el
    /// lance de chica.
    pub fn valor_chica(&self) -> usize {
        (self.0[0].valor() as usize) << 24
            | (self.0[1].valor() as usize) << 16
            | (self.0[2].valor() as usize) << 8
            | self.0[3].valor() as usize
    }

    /// Devuelve el número de parejas de la mano. Si son pares devuelve 1, si son medias devuelve 2
    /// y si son duples 3. En caso de que no haya parejas, devuelve 0.
    pub fn pares(&self) -> Option<Pares> {
        let mut contadores = [0; 13];
        self.0
            .iter()
            .for_each(|c| contadores[c.valor() as usize] += 1);

        let mut grupos = [0; 5];

        contadores
            .iter()
            .enumerate()
            .for_each(|(valor, num)| grupos[*num] |= 1 << valor);

        if grupos[4] > 0 {
            return Some(Pares::Duples(grupos[4] << 1));
        }
        if grupos[3] > 0 {
            return Some(Pares::Medias(grupos[3]));
        }
        if grupos[2] > 0 {
            if grupos[2].count_ones() == 2 {
                return Some(Pares::Duples(grupos[2]));
            }
            return Some(Pares::Pareja(grupos[2]));
        }
        None
    }

    /// Devuelve los puntos de la mano para los lances de punto y juego.
    pub fn valor_puntos(&self) -> u8 {
        self.0.iter().fold(0, |acc, c| {
            if c.valor() >= 10 {
                acc + 10
            } else {
                acc + c.valor()
            }
        })
    }

    pub fn juego(&self) -> Option<Juego> {
        let p = self.valor_puntos();
        match p {
            31 => Some(Juego::Treintayuna),
            32 => Some(Juego::Treintaydos),
            33..=40 => Some(Juego::Resto(p)),
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
        assert_eq!(format!("{m}"), "27C3");
    }

    #[test]
    fn test_codigo() {
        let m = Mano::new(vec![Carta::As, Carta::As, Carta::As, Carta::Tres]);
        assert_eq!(m.valor_grande(), 201392385);
        assert_eq!(m.valor_chica(), 16843020);
    }
}
