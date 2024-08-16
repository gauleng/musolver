use super::mus_error::*;

#[derive(Eq, Debug, Copy, Clone)]
pub enum Carta {
    As = 0,
    Dos = 1,
    Tres = 2,
    Cuatro = 3,
    Cinco = 4,
    Seis = 5,
    Siete = 6,
    Sota = 7,
    Caballo = 8,
    Rey = 9,
}

impl Carta {
    pub fn valor(&self) -> u8 {
        match self {
            Carta::As | Carta::Dos => 1,
            Carta::Cuatro => 4,
            Carta::Cinco => 5,
            Carta::Seis => 6,
            Carta::Siete => 7,
            Carta::Sota => 10,
            Carta::Caballo => 11,
            Carta::Tres | Carta::Rey => 12,
        }
    }
}

impl From<&Carta> for char {
    fn from(other: &Carta) -> char {
        match other {
            Carta::As => '1',
            Carta::Dos => '2',
            Carta::Tres => '3',
            Carta::Cuatro => '4',
            Carta::Cinco => '5',
            Carta::Seis => '6',
            Carta::Siete => '7',
            Carta::Sota => 'S',
            Carta::Caballo => 'C',
            Carta::Rey => 'R',
        }
    }
}

impl TryFrom<char> for Carta {
    type Error = MusError;

    fn try_from(other: char) -> Result<Self, Self::Error> {
        match other {
            'A' | '1' => Ok(Carta::As),
            '2' => Ok(Carta::Dos),
            '3' => Ok(Carta::Tres),
            '4' => Ok(Carta::Cuatro),
            '5' => Ok(Carta::Cinco),
            '6' => Ok(Carta::Seis),
            '7' => Ok(Carta::Siete),
            'S' => Ok(Carta::Sota),
            'C' => Ok(Carta::Caballo),
            'R' => Ok(Carta::Rey),
            _ => Err(MusError::CaracterNoValido(other)),
        }
    }
}

impl PartialEq for Carta {
    fn eq(&self, other: &Self) -> bool {
        self.valor() == other.valor()
    }
}

impl Ord for Carta {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.valor().cmp(&other.valor())
    }
}

impl PartialOrd for Carta {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparar_valor() {
        assert_eq!(Carta::As, Carta::Dos);
        assert_eq!(Carta::Tres, Carta::Rey);
        assert!(Carta::Caballo < Carta::Tres);
    }

    #[test]
    fn try_from_char() {
        let chars = ['A', '1', '2', '3', '4', '5', '6', '7', 'S', 'C', 'R'];
        for c in chars {
            assert!(Carta::try_from(c).is_ok());
        }
        assert!(Carta::try_from('8').is_err());
    }
}
