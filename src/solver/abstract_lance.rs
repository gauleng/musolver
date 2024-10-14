use std::fmt::{Display, Write};

use crate::mus::{Carta, Juego, Mano, Pares};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum AbstractGrande {
    NoCerdos(Carta),
    UnCerdo(Carta),
    DosCerdos(Carta),
    TresCerdos(Carta),
}

impl AbstractGrande {
    pub fn abstract_hand(m: &Mano) -> Self {
        let cartas = m.cartas();
        if cartas[2] == Carta::Rey {
            Self::TresCerdos(cartas[3])
        } else if cartas[1] == Carta::Rey {
            Self::DosCerdos(cartas[2])
        } else if cartas[0] == Carta::Rey {
            Self::UnCerdo(cartas[1])
        } else {
            Self::NoCerdos(cartas[0])
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum AbstractChica {
    NoPitos(Carta),
    UnPito(Carta),
    DosPitos(Carta),
    TresPitos(Carta),
}

impl AbstractChica {
    pub fn abstract_hand(m: &Mano) -> Self {
        let cartas = m.cartas();
        if cartas[1] == Carta::As {
            Self::TresPitos(cartas[0])
        } else if cartas[2] == Carta::As {
            Self::DosPitos(cartas[1])
        } else if cartas[3] == Carta::As {
            Self::UnPito(cartas[1])
        } else {
            Self::NoPitos(cartas[0])
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
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum AbstractPares {
    Pareja(Carta),
    Medias(Carta),
    Duples(Carta, Carta),
}

impl AbstractPares {
    pub fn abstract_hand(m: &Mano) -> Option<Self> {
        m.pares().map(|p| match p {
            Pares::Pareja(v) => {
                let zeros = v.trailing_zeros() as u8;
                Self::Pareja(zeros.try_into().unwrap())
            }
            Pares::Medias(v) => {
                let zeros = v.trailing_zeros() as u8;
                Self::Medias(zeros.try_into().unwrap())
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
                Self::Duples(carta1.try_into().unwrap(), carta2.try_into().unwrap())
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
    pub fn abstract_hand(m: &Mano) -> Option<Self> {
        m.juego().map(|j| match j {
            Juego::Resto(33) => Self::Treintaytres,
            Juego::Resto(34) => {
                if m.num_figuras() == 2 {
                    Self::Treintaycuatro2F
                } else {
                    Self::Treintaycuatro3F
                }
            }
            Juego::Resto(35) => Self::Treintaycinco,
            Juego::Resto(36) => Self::Treintayseis,
            Juego::Resto(37) => Self::Treintaysiete,
            Juego::Resto(40) => Self::Cuarenta,
            Juego::Treintaydos => {
                if m.cartas()[3] == Carta::Cinco {
                    Self::Treintaydos75
                } else {
                    Self::Treintaydos66
                }
            }
            Juego::Treintayuna => match m.num_figuras() {
                1 => Self::Treintayuna1F,
                2 => {
                    if m.cartas()[3] == Carta::Cuatro {
                        Self::Treintayuna2F74
                    } else {
                        Self::Treintayuna2F65
                    }
                }
                3 => Self::Treintayuna3F,
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

pub enum AbstractPunto {
    Punto(u8),
}

impl AbstractPunto {
    pub fn abstract_hand(m: &Mano) -> Self {
        Self::Punto(m.valor_puntos())
    }
}

impl Display for AbstractPunto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Punto(valor) => write!(f, "{valor}"),
        }
    }
}
