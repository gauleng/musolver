use std::fmt::{Display, Write};

use crate::mus::{Carta, Juego, Mano, Pares};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub enum AbstractJugada {
    AbstractGrande(AbstractGrande),
    AbstractChica(AbstractChica),
    AbstractPares(AbstractPares),
    AbstractJuego(AbstractJuego),
    AbstractPunto(AbstractPunto),
}

impl Display for AbstractJugada {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AbstractJugada::AbstractGrande(grande) => grande.fmt(f),
            AbstractJugada::AbstractChica(chica) => chica.fmt(f),
            AbstractJugada::AbstractPares(pares) => pares.fmt(f),
            AbstractJugada::AbstractJuego(juego) => juego.fmt(f),
            AbstractJugada::AbstractPunto(punto) => punto.fmt(f),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum AbstractGrande {
    NoCerdos(Carta),
    UnCerdo(Carta),
    DosCerdos(Carta),
    TresCerdos(Carta),
}

impl AbstractGrande {
    pub fn abstract_hand(m: &Mano) -> AbstractJugada {
        let cartas = m.cartas();
        if cartas[2] == Carta::Rey {
            AbstractJugada::AbstractGrande(Self::TresCerdos(cartas[3]))
        } else if cartas[1] == Carta::Rey {
            AbstractJugada::AbstractGrande(Self::DosCerdos(cartas[2]))
        } else if cartas[0] == Carta::Rey {
            AbstractJugada::AbstractGrande(Self::UnCerdo(cartas[1]))
        } else {
            AbstractJugada::AbstractGrande(Self::NoCerdos(cartas[0]))
        }
    }
}

impl Display for AbstractGrande {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoCerdos(carta) => f.write_char(carta.into()),
            Self::UnCerdo(carta) => {
                f.write_char('R')?;
                f.write_char(carta.into())
            }
            Self::DosCerdos(carta) => {
                f.write_str("RR")?;
                f.write_char(carta.into())
            }
            Self::TresCerdos(carta) => {
                f.write_str("RRR")?;
                f.write_char(carta.into())
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum AbstractChica {
    NoPitos(Carta),
    UnPito(Carta),
    DosPitos(Carta),
    TresPitos(Carta),
}

impl AbstractChica {
    pub fn abstract_hand(m: &Mano) -> AbstractJugada {
        let cartas = m.cartas();
        if cartas[1] == Carta::As {
            AbstractJugada::AbstractChica(Self::TresPitos(cartas[0]))
        } else if cartas[2] == Carta::As {
            AbstractJugada::AbstractChica(Self::DosPitos(cartas[1]))
        } else if cartas[3] == Carta::As {
            AbstractJugada::AbstractChica(Self::UnPito(cartas[2]))
        } else {
            AbstractJugada::AbstractChica(Self::NoPitos(cartas[3]))
        }
    }

    fn value(&self) -> u8 {
        match self {
            AbstractChica::NoPitos(carta) => 48 + carta.valor(),
            AbstractChica::UnPito(carta) => 32 + carta.valor(),
            AbstractChica::DosPitos(carta) => 16 + carta.valor(),
            AbstractChica::TresPitos(carta) => carta.valor(),
        }
    }
}

impl Display for AbstractChica {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPitos(carta) => f.write_char(carta.into()),
            Self::UnPito(carta) => {
                f.write_char(carta.into())?;
                f.write_char('1')
            }
            Self::DosPitos(carta) => {
                f.write_char(carta.into())?;
                f.write_str("11")
            }
            Self::TresPitos(carta) => {
                f.write_char(carta.into())?;
                f.write_str("111")
            }
        }
    }
}

impl Ord for AbstractChica {
    fn cmp(&self, other: &AbstractChica) -> std::cmp::Ordering {
        other.value().cmp(&self.value())
    }
}

impl PartialOrd for AbstractChica {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum AbstractPares {
    Pareja(Carta),
    Medias(Carta),
    Duples(Carta, Carta),
}

impl AbstractPares {
    pub fn abstract_hand(m: &Mano) -> Option<AbstractJugada> {
        m.pares().map(|p| match p {
            Pares::Pareja(v) => {
                let zeros = v.trailing_zeros() as u8;
                AbstractJugada::AbstractPares(Self::Pareja(zeros.try_into().unwrap()))
            }
            Pares::Medias(v) => {
                let zeros = v.trailing_zeros() as u8;
                AbstractJugada::AbstractPares(Self::Medias(zeros.try_into().unwrap()))
            }
            Pares::Duples(v) => {
                let (carta1, carta2) = if v.count_ones() == 2 {
                    ((15 - v.leading_zeros()) as u8, v.trailing_zeros() as u8)
                } else {
                    (
                        (v.trailing_zeros() - 1) as u8,
                        (v.trailing_zeros() - 1) as u8,
                    )
                };
                AbstractJugada::AbstractPares(Self::Duples(
                    carta1.try_into().unwrap(),
                    carta2.try_into().unwrap(),
                ))
            }
        })
    }
}

impl Display for AbstractPares {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pareja(carta) => {
                f.write_char('P')?;
                f.write_char(carta.into())
            }
            Self::Medias(carta) => {
                f.write_char('M')?;
                f.write_char(carta.into())
            }
            Self::Duples(carta1, carta2) => {
                f.write_char('D')?;
                f.write_char(carta1.into())?;
                f.write_char(':')?;
                f.write_char(carta2.into())
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum AbstractJuego {
    Treintaytres,
    Treintaycuatro2F,
    Treintaycuatro3F,
    Treintaycinco,
    Treintayseis,
    Treintaysiete,
    Cuarenta,
    Treintaydos66,
    Treintaydos75,
    Treintayuna1F,
    Treintayuna2F74,
    Treintayuna2F65,
    Treintayuna3F,
}

impl AbstractJuego {
    pub fn abstract_hand(m: &Mano) -> Option<AbstractJugada> {
        m.juego().map(|j| match j {
            Juego::Resto(33) => AbstractJugada::AbstractJuego(Self::Treintaytres),
            Juego::Resto(34) => {
                if m.num_figuras() == 2 {
                    AbstractJugada::AbstractJuego(Self::Treintaycuatro2F)
                } else {
                    AbstractJugada::AbstractJuego(Self::Treintaycuatro3F)
                }
            }
            Juego::Resto(35) => AbstractJugada::AbstractJuego(Self::Treintaycinco),
            Juego::Resto(36) => AbstractJugada::AbstractJuego(Self::Treintayseis),
            Juego::Resto(37) => AbstractJugada::AbstractJuego(Self::Treintaysiete),
            Juego::Resto(40) => AbstractJugada::AbstractJuego(Self::Cuarenta),
            Juego::Treintaydos => {
                if m.cartas()[3] == Carta::Cinco {
                    AbstractJugada::AbstractJuego(Self::Treintaydos75)
                } else {
                    AbstractJugada::AbstractJuego(Self::Treintaydos66)
                }
            }
            Juego::Treintayuna => match m.num_figuras() {
                1 => AbstractJugada::AbstractJuego(Self::Treintayuna1F),
                2 => {
                    if m.cartas()[3] == Carta::Cuatro {
                        AbstractJugada::AbstractJuego(Self::Treintayuna2F74)
                    } else {
                        AbstractJugada::AbstractJuego(Self::Treintayuna2F65)
                    }
                }
                3 => AbstractJugada::AbstractJuego(Self::Treintayuna3F),
                _ => panic!("No existen 31 que no sean de 1, 2 o 3 figuras."),
            },
            _ => panic!("Valor de juego incorrecto"),
        })
    }
}

impl Display for AbstractJuego {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Treintaytres => write!(f, "33"),
            Self::Treintaycuatro2F => write!(f, "34XX77"),
            Self::Treintaycuatro3F => write!(f, "34XXX4"),
            Self::Treintaycinco => write!(f, "35"),
            Self::Treintayseis => write!(f, "36"),
            Self::Treintaysiete => write!(f, "37"),
            Self::Cuarenta => write!(f, "40"),
            Self::Treintaydos66 => write!(f, "32XX66"),
            Self::Treintaydos75 => write!(f, "32XX75"),
            Self::Treintayuna1F => write!(f, "31X777"),
            Self::Treintayuna2F65 => write!(f, "31XX65"),
            Self::Treintayuna2F74 => write!(f, "31XX74"),
            Self::Treintayuna3F => write!(f, "31XXX1"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub enum AbstractPunto {
    Punto(u8),
}

impl AbstractPunto {
    pub fn abstract_hand(m: &Mano) -> AbstractJugada {
        AbstractJugada::AbstractPunto(Self::Punto(m.valor_puntos()))
    }
}

impl Display for AbstractPunto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Punto(valor) => write!(f, "{valor}"),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_abstract_grande() {
        assert_eq!(
            AbstractGrande::abstract_hand(&"RRR1".try_into().unwrap()),
            AbstractJugada::AbstractGrande(AbstractGrande::TresCerdos(Carta::As))
        );
        assert_eq!(
            AbstractGrande::abstract_hand(&"RR11".try_into().unwrap()),
            AbstractJugada::AbstractGrande(AbstractGrande::DosCerdos(Carta::As))
        );
        assert_eq!(
            AbstractGrande::abstract_hand(&"R111".try_into().unwrap()),
            AbstractJugada::AbstractGrande(AbstractGrande::UnCerdo(Carta::As))
        );
        assert_eq!(
            AbstractGrande::abstract_hand(&"C411".try_into().unwrap()),
            AbstractJugada::AbstractGrande(AbstractGrande::NoCerdos(Carta::Caballo))
        );
    }

    #[test]
    fn test_abstract_chica() {
        assert_eq!(
            AbstractChica::abstract_hand(&"C111".try_into().unwrap()),
            AbstractJugada::AbstractChica(AbstractChica::TresPitos(Carta::Caballo))
        );
        assert_eq!(
            AbstractChica::abstract_hand(&"CC11".try_into().unwrap()),
            AbstractJugada::AbstractChica(AbstractChica::DosPitos(Carta::Caballo))
        );
        assert_eq!(
            AbstractChica::abstract_hand(&"CCC1".try_into().unwrap()),
            AbstractJugada::AbstractChica(AbstractChica::UnPito(Carta::Caballo))
        );
        assert_eq!(
            AbstractChica::abstract_hand(&"CCCC".try_into().unwrap()),
            AbstractJugada::AbstractChica(AbstractChica::NoPitos(Carta::Caballo))
        );
    }

    #[test]
    fn test_abstract_pares() {
        assert_eq!(
            AbstractPares::abstract_hand(&"CCCC".try_into().unwrap()),
            Some(AbstractJugada::AbstractPares(AbstractPares::Duples(
                Carta::Caballo,
                Carta::Caballo
            )))
        );
        assert_eq!(
            AbstractPares::abstract_hand(&"RR11".try_into().unwrap()),
            Some(AbstractJugada::AbstractPares(AbstractPares::Duples(
                Carta::Rey,
                Carta::As
            )))
        );
    }

    #[test]
    fn test_abstract_juego() {
        assert_eq!(
            AbstractJuego::abstract_hand(&"CCCC".try_into().unwrap()),
            Some(AbstractJugada::AbstractJuego(AbstractJuego::Cuarenta))
        );
        assert_eq!(
            AbstractJuego::abstract_hand(&"RC74".try_into().unwrap()),
            Some(AbstractJugada::AbstractJuego(
                AbstractJuego::Treintayuna2F74
            ))
        );
    }

    #[test]
    fn test_abstract_punto() {
        assert_eq!(
            AbstractPunto::abstract_hand(&"C111".try_into().unwrap()),
            AbstractJugada::AbstractPunto(AbstractPunto::Punto(13))
        );
    }
}
