use clap::ValueEnum;
use serde::Deserialize;
use serde::Serialize;

use super::MusError;
use crate::mus::Accion;
use crate::mus::Mano;

use std::cmp;
use std::fmt::Display;

/// Jugadas del lance juego.
#[derive(Hash, Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
pub enum Juego {
    Resto(u8),
    Treintaydos,
    Treintayuna,
}

impl Display for Juego {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Juego::Resto(v) => write!(f, "{v}"),
            Juego::Treintaydos => write!(f, "32"),
            Juego::Treintayuna => write!(f, "31"),
        }
    }
}

/// Jugadas del lance pares.
#[derive(Hash, Debug, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
pub enum Pares {
    Pareja(u16),
    Medias(u16),
    Duples(u16),
}

impl Display for Pares {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pares::Pareja(v) => write!(f, "P{}", v.trailing_zeros()),
            Pares::Medias(v) => write!(f, "M{}", v.trailing_zeros()),
            Pares::Duples(v) => {
                if v.count_ones() == 2 {
                    write!(f, "D{}:{}", 15 - v.leading_zeros(), v.trailing_zeros())
                } else {
                    write!(f, "D{}:{}", v.trailing_zeros() - 1, v.trailing_zeros() - 1)
                }
            }
        }
    }
}

/// Lances de una partida de mus.
#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug, ValueEnum, Serialize, Deserialize)]
pub enum Lance {
    Grande,
    Chica,
    Pares,
    Punto,
    Juego,
}

impl Mano {
    /// Convierte la mano a un entero de 4 bytes. La cuarta carta se mapea al primer byte, la
    /// tercera al segundo byte, y así sucesivamente. Este valor permite ordenar las manos en el
    /// lance de grande.
    pub fn valor_grande(&self) -> usize {
        let c = self.cartas();
        (c[0].valor() as usize) << 24
            | (c[1].valor() as usize) << 16
            | (c[2].valor() as usize) << 8
            | c[3].valor() as usize
    }

    /// Convierte la mano a un entero de 4 bytes. La primera carta se mapea al primer byte, la
    /// segunda al segundo byte, y así sucesivamente. Este valor permite ordenar las manos en el
    /// lance de chica.
    pub fn valor_chica(&self) -> usize {
        let c = self.cartas();
        (c[3].valor() as usize) << 24
            | (c[2].valor() as usize) << 16
            | (c[1].valor() as usize) << 8
            | c[0].valor() as usize
    }

    /// Indica si la mano tiene una jugada para el lance de Pares, y en caso contrario devuelve
    /// None.
    pub fn pares(&self) -> Option<Pares> {
        let mut contadores = [0; 13];
        let c = self.cartas();
        c.iter().for_each(|c| contadores[c.valor() as usize] += 1);

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
        let c = self.cartas();
        c.iter().fold(0, |acc, c| {
            if c.valor() >= 10 {
                acc + 10
            } else {
                acc + c.valor()
            }
        })
    }

    /// Indica si la mano tiene jugada para el lance de juego. En caso contrario devuelve None.
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

pub trait RankingManos {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering;
    ///
    /// Dado un vector de manos de mus, devuelve el índice de la mejor de ellas dado el lance en
    /// juego. Se asume que la primera mano del vector es la del jugador mano y la última la del
    /// postre.
    fn mejor_mano(&self, manos: &[Mano]) -> usize {
        let m = manos
            .iter()
            .enumerate()
            .rev()
            .max_by(|i, j| self.compara_manos(i.1, j.1));
        m.unwrap().0
    }
}

impl RankingManos for Lance {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        match self {
            Lance::Grande => a.valor_grande().cmp(&b.valor_grande()),
            Lance::Chica => b.valor_chica().cmp(&a.valor_chica()),
            Lance::Pares => a.pares().cmp(&b.pares()),
            Lance::Juego => a.juego().cmp(&b.juego()),
            Lance::Punto => a.valor_puntos().cmp(&b.valor_puntos()),
        }
    }
}

impl Lance {
    /// Tantos asociados al valor de una mano. En los lances de pares y juego se corresponde con el
    /// tanteo de las jugadas y en el resto de lances es cero.
    pub fn tantos_mano(&self, mano: &Mano) -> u8 {
        match self {
            Lance::Pares => match mano.pares() {
                Some(Pares::Pareja(_)) => 1,
                Some(Pares::Medias(_)) => 2,
                Some(Pares::Duples(_)) => 3,
                None => 0,
            },
            Lance::Juego => match mano.juego() {
                Some(Juego::Treintayuna) => 3,
                Some(Juego::Treintaydos) | Some(Juego::Resto(_)) => 2,
                None => 0,
            },
            _ => 0,
        }
    }

    /// Tantos apostados cuando todos los jugadores pasan. Es 1 para los lances de grande y chica y
    /// 0 para el resto.
    pub fn apuesta_minima(&self) -> u8 {
        if let Lance::Grande | Lance::Chica = self {
            1
        } else {
            0
        }
    }

    /// Tantos extra recibidos por ganar el lance. Es 1 para el lance de punto y 0 para el resto.
    pub fn bonus(&self) -> u8 {
        if let Lance::Punto = self {
            1
        } else {
            0
        }
    }

    fn hay_lance_jugadas<T>(&self, manos: &[Option<T>]) -> bool {
        manos.iter().any(|j| j.is_some())
    }

    fn jugadas<T, F>(&self, manos: &[Mano], f: F) -> Vec<Option<T>>
    where
        F: Fn(&Mano) -> Option<T>,
    {
        manos.iter().map(f).collect()
    }

    /// Indica qué pareja es la primera en actuar. Por lo general será la pareja del jugador mano,
    /// pero en los lances de pares y juego puede darse la situación contraria en dos casos: que
    /// solo tengan jugada el jugador a la derecha del mano y la pareja del mano, o que el único
    /// sin jugada sea el jugador postre.
    ///
    /// En este último caso, el jugador de la pareja postre está intercalado entre los dos rivales.
    /// Se asume que en esa configuración el jugador mano siempre va a pasar a la espera de lo que
    /// decida su compañero, por lo que empezaría hablando el jugador que está solo.
    pub fn turno_inicial(&self, manos: &[Mano]) -> usize {
        match self {
            Lance::Grande | Lance::Chica | Lance::Punto => 0,
            Lance::Pares => {
                if manos[3].pares().is_some() {
                    return 0;
                }
                if manos[1].pares().is_some() && manos[2].pares().is_some() {
                    1
                } else {
                    0
                }
            }
            Lance::Juego => {
                if manos[3].juego().is_some() {
                    return 0;
                }
                if manos[1].juego().is_some() && manos[2].juego().is_some() {
                    1
                } else {
                    0
                }
            }
        }
    }

    /// Devuelve un bool indicando si el lance tiene lugar, independientemente de si hay lugar a
    /// envites. En los lances de grande y chica siempre devuelve true, en pares y juego
    /// si algún jugador con jugada, y en punto si no hay ningún jugador con juego.
    pub fn hay_lance(&self, manos: &[Mano]) -> bool {
        match self {
            Lance::Grande | Lance::Chica => true,
            Lance::Pares => manos.iter().map(|m| m.pares().is_some()).any(|b| b),
            Lance::Juego => manos.iter().map(|m| m.juego().is_some()).any(|b| b),
            Lance::Punto => !Lance::Juego.hay_lance(manos),
        }
    }

    /// Devuelve un bool indicando si el lance tiene envites. Para ello es necesario que al menos
    /// un jugador de cada pareja tenga jugada.
    pub fn se_juega(&self, manos: &[Mano]) -> bool {
        match self {
            Lance::Grande | Lance::Chica => true,
            Lance::Pares => {
                (manos[0].pares().is_some() || manos[2].pares().is_some())
                    && (manos[1].pares().is_some() || manos[3].pares().is_some())
            }
            Lance::Juego => {
                (manos[0].juego().is_some() || manos[2].juego().is_some())
                    && (manos[1].juego().is_some() || manos[3].juego().is_some())
            }
            Lance::Punto => !self.hay_lance_jugadas(&self.jugadas(manos, |m| m.juego())),
        }
    }
}

// Representa los tantos totales apostados en un lance.
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
pub enum Apuesta {
    Tantos(u8),
    Ordago,
}

#[derive(Debug, Clone)]
pub struct EstadoLance {
    bote: [Apuesta; 2],
    activos: [bool; 2],
    turno: Option<usize>,
    ultimo_envite: usize,
    apuesta_maxima: u8,
    apuesta_minima: u8,
    ganador: Option<usize>,
    jugador_mejor_mano: usize,
}

impl EstadoLance {
    /// Crea un nuevo lance con las apuestas minima y máximas indicadas, la pareja que empieza
    /// actuando y el jugador que tiene la mejor mano.
    pub fn new(
        apuesta_minima: u8,
        apuesta_maxima: u8,
        turno_inicial: usize,
        jugador_mejor_mano: usize,
    ) -> Self {
        EstadoLance {
            bote: [Apuesta::Tantos(0), Apuesta::Tantos(0)],
            activos: [true, true],
            turno: Some(turno_inicial),
            ultimo_envite: turno_inicial,
            apuesta_minima,
            apuesta_maxima,
            ganador: None,
            jugador_mejor_mano,
        }
    }

    /// Efectúa la acción para el jugador del turno actual.
    /// Devuelve el turno del siguiente jugador o None si la ronda de envites acabó.
    /// Devuelve un error si se intenta actuar cuando ya ha terminado la ronda de envites.
    pub fn actuar(&mut self, a: Accion) -> Result<Option<usize>, MusError> {
        let turno = self.turno.ok_or(MusError::AccionNoValida)?;
        let ultimo_bote = self.bote[1];
        match a {
            Accion::Paso => {
                if self.bote[1] > Apuesta::Tantos(0) {
                    self.activos[turno] = false;
                }
            }
            Accion::Quiero => {}
            Accion::Envido(n) => {
                if ultimo_bote < Apuesta::Tantos(self.apuesta_maxima) {
                    if let Apuesta::Tantos(t) = ultimo_bote {
                        let nuevo_bote = Apuesta::Tantos((t + n.max(2)).min(self.apuesta_maxima));
                        self.bote[0] = self.bote[1];
                        self.bote[1] = nuevo_bote;
                        self.ultimo_envite = turno;
                    }
                }
            }
            Accion::Ordago => {
                if ultimo_bote < Apuesta::Tantos(self.apuesta_maxima) {
                    self.bote[0] = self.bote[1];
                    self.bote[1] = Apuesta::Ordago;
                    self.ultimo_envite = turno;
                }
            }
        }
        self.turno = self.pasar_turno();
        if self.turno.is_none() && !self.se_quieren() {
            self.ganador = if self.activos[0] { Some(0) } else { Some(1) };
        }
        Ok(self.turno)
    }

    fn pasar_turno(&self) -> Option<usize> {
        let turno = self.turno?;
        // let num_jugadores = self.activos.len();
        let nuevo_turno = 1 - turno;
        if self.ultimo_envite == nuevo_turno {
            return None;
        }
        if self.activos[nuevo_turno] {
            return Some(nuevo_turno);
        }
        None
        // loop {
        //     nuevo_turno = (nuevo_turno + 1) % num_jugadores;
        //     if nuevo_turno == turno || self.ultimo_envite == nuevo_turno {
        //         return None;
        //     }
        //     if self.activos[nuevo_turno] {
        //         return Some(nuevo_turno);
        //     }
        // }
    }

    fn se_quieren(&self) -> bool {
        self.turno.is_none() && self.activos.iter().all(|b| *b)
    }

    /// Devuelve los tantos totales apostados en el lance hasta el momento. Se considera apostada
    /// cualquier cantidad que se quiso o que se ha aceptado tácitamente con un nuevo envite. En
    /// caso de que rechace un envite sin que hubiera en ese momento tantos apostados, se asume que
    /// se apuesta un tanto. Si el lance tiene apuesta mínima y todos los jugadores pasaron, se
    /// asume que se apuesta esa cantidad. Esto último solo ocurre en grande y chica.
    pub fn tantos_apostados(&self) -> Apuesta {
        let mut apostado = if self.se_quieren() {
            self.bote[1]
        } else if self.bote[0] == Apuesta::Tantos(0) && self.bote[1] > Apuesta::Tantos(0) {
            Apuesta::Tantos(1)
        } else {
            self.bote[0]
        };
        if apostado == Apuesta::Tantos(0) {
            apostado = Apuesta::Tantos(self.apuesta_minima);
        }
        apostado
    }

    /// Determina el ganador del lance. Si no se quisieron, devuelve la pareja que se lleva los
    /// tantos. En caso contrario, resuelve el lance con las manos recibidas.
    /// Si el lance está resuelto y se vuelve a llamar a esta función, devolverá el mismo ganador
    /// ya calculado anteriormente. Devuelve el número de pareja que ha ganado el lance.
    pub fn resolver_lance(&mut self) -> usize {
        self.turno = None;
        *self.ganador.get_or_insert(self.jugador_mejor_mano % 2)
    }

    /// India si ya hay un ganador en el lance, bien sea porque una pareja ha rechazado un envite o
    /// porque el lance ya está resuelto. En caso de que todavía un no haya ganador, esta función
    /// devuelve None.
    pub fn ganador(&self) -> Option<usize> {
        self.ganador
    }

    /// Devuelve el turno de la pareja que le toca actuar. En caso de que el lance ya haya acabado
    /// devuelve None.
    pub fn turno(&self) -> Option<usize> {
        self.turno
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turno() {
        let mut partida = EstadoLance::new(0, 40, 0, 0);
        assert_eq!(partida.turno(), Some(0));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), Some(1));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), None);

        let mut partida = EstadoLance::new(0, 40, 1, 0);
        assert_eq!(partida.turno(), Some(1));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), Some(0));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), None);
    }

    #[test]
    fn test_turno2() {
        let mut partida = EstadoLance::new(0, 40, 0, 0);
        assert_eq!(partida.actuar(Accion::Envido(2)).unwrap(), Some(1));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), None);
    }

    #[test]
    fn test_ganador() {
        let manos = vec![
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0, Lance::Grande.mejor_mano(&manos));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance();
        assert_eq!(partida.ganador(), Some(0));

        let manos = vec![
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RRRR").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0, Lance::Grande.mejor_mano(&manos));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance();
        assert_eq!(partida.ganador(), Some(0));

        let manos = vec![
            Mano::try_from("RRRC").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0, Lance::Grande.mejor_mano(&manos));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance();
        assert_eq!(partida.ganador(), Some(1));

        let manos = vec![
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("1111").unwrap(),
            Mano::try_from("1111").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0, Lance::Grande.mejor_mano(&manos));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance();
        assert_eq!(partida.ganador(), Some(0));

        let manos = vec![
            Mano::try_from("1111").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("1111").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0, Lance::Grande.mejor_mano(&manos));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance();
        assert_eq!(partida.ganador(), Some(1));

        let manos = vec![
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RR11").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("R111").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0, Lance::Grande.mejor_mano(&manos));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance();
        assert_eq!(partida.ganador(), Some(1));

        let manos = vec![
            Mano::try_from("R111").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RR11").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0, Lance::Grande.mejor_mano(&manos));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance();
        assert_eq!(partida.ganador(), Some(1));

        let manos = vec![
            Mano::try_from("R111").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RR11").unwrap(),
            Mano::try_from("R111").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0, Lance::Grande.mejor_mano(&manos));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance();
        assert_eq!(partida.ganador(), Some(0));
    }

    #[test]
    fn hay_lance() {
        let manos = vec![
            Mano::try_from("R111").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RR11").unwrap(),
            Mano::try_from("R111").unwrap(),
        ];
        assert!(Lance::Grande.hay_lance(&manos));
        assert!(Lance::Chica.hay_lance(&manos));
        assert!(Lance::Pares.hay_lance(&manos));
        assert!(Lance::Pares.se_juega(&manos));
        assert!(!Lance::Juego.hay_lance(&manos));
        assert!(Lance::Punto.hay_lance(&manos));

        let manos = vec![
            Mano::try_from("R571").unwrap(),
            Mano::try_from("R571").unwrap(),
            Mano::try_from("RR11").unwrap(),
            Mano::try_from("R751").unwrap(),
        ];
        assert!(Lance::Pares.hay_lance(&manos));
        assert!(!Lance::Pares.se_juega(&manos));

        let manos = vec![
            Mano::try_from("R571").unwrap(),
            Mano::try_from("R571").unwrap(),
            Mano::try_from("R571").unwrap(),
            Mano::try_from("R751").unwrap(),
        ];
        assert!(!Lance::Pares.hay_lance(&manos));
        assert!(!Lance::Pares.se_juega(&manos));

        let manos = vec![
            Mano::try_from("RCS1").unwrap(),
            Mano::try_from("RCS1").unwrap(),
            Mano::try_from("RR11").unwrap(),
            Mano::try_from("RCS1").unwrap(),
        ];
        assert!(Lance::Juego.hay_lance(&manos));
        assert!(Lance::Juego.se_juega(&manos));

        let manos = vec![
            Mano::try_from("RCS1").unwrap(),
            Mano::try_from("R1S1").unwrap(),
            Mano::try_from("RRC1").unwrap(),
            Mano::try_from("R1S1").unwrap(),
        ];
        assert!(Lance::Juego.hay_lance(&manos));
        assert!(!Lance::Juego.se_juega(&manos));
        assert!(!Lance::Punto.hay_lance(&manos));
        assert!(!Lance::Punto.se_juega(&manos));
    }

    #[test]
    fn test_turno_inicial() {
        let manos = vec![
            Mano::try_from("R1S1").unwrap(),
            Mano::try_from("R1S1").unwrap(),
            Mano::try_from("RRC1").unwrap(),
            Mano::try_from("RCS1").unwrap(),
        ];
        assert_eq!(Lance::Grande.turno_inicial(&manos), 0);
        assert_eq!(Lance::Chica.turno_inicial(&manos), 0);
        assert_eq!(Lance::Pares.turno_inicial(&manos), 1);
        assert_eq!(Lance::Juego.turno_inicial(&manos), 0);
    }

    #[test]
    fn test_tanteo() {
        let mut e = EstadoLance::new(0, 40, 0, 0);
        let _ = e.actuar(Accion::Paso);
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), None);
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(0));

        let mut e = EstadoLance::new(1, 40, 0, 0);
        let _ = e.actuar(Accion::Paso);
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), None);
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(1));

        let mut e = EstadoLance::new(0, 40, 0, 0);
        let _ = e.actuar(Accion::Envido(2));
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), Some(0));
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(1));

        let mut e = EstadoLance::new(1, 40, 0, 0);
        let _ = e.actuar(Accion::Envido(2));
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), Some(0));
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(1));

        let mut e = EstadoLance::new(0, 40, 0, 0);
        let _ = e.actuar(Accion::Paso);
        let _ = e.actuar(Accion::Envido(2));
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), Some(1));
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(1));

        let mut e = EstadoLance::new(1, 40, 0, 0);
        let _ = e.actuar(Accion::Paso);
        let _ = e.actuar(Accion::Envido(2));
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), Some(1));
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(1));
    }
    use std::cmp::Ordering::*;

    #[test]
    fn orden_jugadas() {
        assert!(Juego::Treintayuna > Juego::Resto(33));
        assert!(Juego::Treintayuna > Juego::Treintaydos);
        let p1 = Mano::try_from("1111").unwrap().pares();
        let p2 = Mano::try_from("1144").unwrap().pares();
        let p3 = Mano::try_from("4444").unwrap().pares();
        assert!(p2 > p1);
        assert!(p3 > p2);
        let p4 = Mano::try_from("CC55").unwrap().pares();
        let p5 = Mano::try_from("RRR1").unwrap().pares();
        assert!(p4 > p5);
        assert!(p4 > p2);
        assert!(p1 > p5);
    }

    #[test]
    fn test_compara_manos1() {
        let a = Mano::try_from("355R").unwrap();
        let b = Mano::try_from("3555").unwrap();
        let grande = Lance::Grande;
        let chica = Lance::Chica;
        let pares = Lance::Pares;
        let juego = Lance::Juego;
        let punto = Lance::Punto;
        assert_eq!(grande.compara_manos(&a, &b), Greater);
        assert_eq!(chica.compara_manos(&a, &b), Less);
        assert_eq!(pares.compara_manos(&a, &b), Greater);
        assert_eq!(punto.compara_manos(&a, &b), Greater);
        assert_eq!(juego.compara_manos(&a, &b), Equal);
        let manos = vec![a, b];
        assert_eq!(chica.mejor_mano(&manos), 1);
    }

    #[test]
    fn test_compara_manos2() {
        let a = Mano::try_from("1147").unwrap();
        let b = Mano::try_from("1247").unwrap();
        let grande = Lance::Grande;
        let chica = Lance::Chica;
        let pares = Lance::Pares;
        let juego = Lance::Juego;
        let punto = Lance::Punto;
        assert_eq!(grande.compara_manos(&a, &b), Equal);
        assert_eq!(chica.compara_manos(&a, &b), Equal);
        assert_eq!(pares.compara_manos(&a, &b), Equal);
        assert_eq!(punto.compara_manos(&a, &b), Equal);
        assert_eq!(juego.compara_manos(&a, &b), Equal);
        let manos = vec![a, b];
        assert_eq!(grande.mejor_mano(&manos), 0);
    }

    #[test]
    fn test_compara_manos3() {
        let a = Mano::try_from("2CRR").unwrap();
        let b = Mano::try_from("SSCR").unwrap();
        let grande = Lance::Grande;
        let chica = Lance::Chica;
        let pares = Lance::Pares;
        let juego = Lance::Juego;
        let punto = Lance::Punto;
        assert_eq!(grande.compara_manos(&a, &b), Greater);
        assert_eq!(chica.compara_manos(&a, &b), Greater);
        assert_eq!(pares.compara_manos(&a, &b), Greater);
        assert_eq!(punto.compara_manos(&a, &b), Less);
        assert_eq!(juego.compara_manos(&a, &b), Greater);
        let manos = vec![a, b];
        assert_eq!(juego.mejor_mano(&manos), 0);
    }
}
