use super::MusError;
use crate::mus::Accion;
use crate::mus::Mano;

use std::cmp;

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

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
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
        (c[3].valor() as usize) << 24
            | (c[2].valor() as usize) << 16
            | (c[1].valor() as usize) << 8
            | c[0].valor() as usize
    }

    /// Convierte la mano a un entero de 4 bytes. La primera carta se mapea al primer byte, la
    /// segunda al segundo byte, y así sucesivamente. Este valor permite ordenar las manos en el
    /// lance de chica.
    pub fn valor_chica(&self) -> usize {
        let c = self.cartas();
        (c[0].valor() as usize) << 24
            | (c[1].valor() as usize) << 16
            | (c[2].valor() as usize) << 8
            | c[3].valor() as usize
    }

    /// Devuelve el número de parejas de la mano. Si son pares devuelve 1, si son medias devuelve 2
    /// y si son duples 3. En caso de que no haya parejas, devuelve 0.
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
        let mut indices: Vec<usize> = (0..manos.len()).rev().collect();
        indices.sort_by(|i, j| self.compara_manos(&manos[*i], &manos[*j]));
        *indices.last().unwrap()
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
}

pub struct EstadoLance {
    bote: [u8; 2],
    activos: [bool; 2],
    turno: Option<usize>,
    ultimo_envite: usize,
    apuesta_maxima: u8,
    ganador: Option<usize>,
}

impl EstadoLance {
    pub fn new(apuesta_maxima: u8) -> Self {
        EstadoLance {
            bote: [0, 0],
            activos: [true, true],
            turno: Some(0),
            ultimo_envite: 0,
            apuesta_maxima,
            ganador: None,
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
                if self.bote[1] > 0 {
                    self.activos[turno] = false;
                }
            }
            Accion::Quiero => {}
            Accion::Envido(n) => {
                if ultimo_bote < self.apuesta_maxima {
                    let nuevo_bote = (ultimo_bote + n.max(2)).min(self.apuesta_maxima);
                    self.bote[0] = self.bote[1];
                    self.bote[1] = nuevo_bote;
                    self.ultimo_envite = turno;
                }
            }
            Accion::Ordago => {
                if ultimo_bote < self.apuesta_maxima {
                    self.bote[0] = self.bote[1];
                    self.bote[1] = self.apuesta_maxima;
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

    fn tantos_apostados(&self) -> u8 {
        if self.se_quieren() {
            self.bote[1]
        } else {
            self.bote[0]
        }
    }

    pub fn resolver_lance<L>(&mut self, manos: &[Mano], lance: &L) -> Option<usize>
    where
        L: RankingManos,
    {
        self.ganador = Some(lance.mejor_mano(manos) % 2);
        self.ganador
    }

    pub fn ganador(&self) -> Option<usize> {
        self.ganador
    }

    pub fn tantos(&mut self, manos: &[Mano], lance: &Lance) -> Option<Vec<u8>> {
        if self.turno.is_some() {
            return None;
        }
        let mut apostado = self.tantos_apostados();
        if let Lance::Grande | Lance::Chica = lance {
            if apostado == 0 {
                apostado = 1;
            }
        }

        let ganador = self
            .ganador()
            .or_else(|| self.resolver_lance(manos, lance))
            .unwrap();
        let manos_ganadoras = [ganador, ganador + 2];

        let mut tantos = vec![0, 0];
        tantos[ganador] = apostado;
        tantos[ganador] += lance.tantos_mano(&manos[manos_ganadoras[0]])
            + lance.tantos_mano(&manos[manos_ganadoras[1]]);
        if let Lance::Punto = lance {
            tantos[ganador] += 1;
        }

        Some(tantos)
    }

    pub fn turno(&self) -> Option<usize> {
        self.turno
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turno() {
        let mut partida = EstadoLance::new(40);
        assert_eq!(partida.turno(), Some(0));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), Some(1));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), None);
    }

    #[test]
    fn test_turno2() {
        let mut partida = EstadoLance::new(40);
        assert_eq!(partida.actuar(Accion::Envido(2)).unwrap(), Some(1));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), None);
    }

    #[test]
    fn test_tanteo() {
        let manos = vec![
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];

        let mut partida = EstadoLance::new(40);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(&manos, &Lance::Grande).unwrap(), vec![1, 0]);
        let mut partida = EstadoLance::new(40);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(&manos, &Lance::Chica).unwrap(), vec![1, 0]);
        let mut partida = EstadoLance::new(40);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(&manos, &Lance::Pares).unwrap(), vec![3, 0]);
        let mut partida = EstadoLance::new(40);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(&manos, &Lance::Juego).unwrap(), vec![0, 2]);
    }

    #[test]
    fn test_tanteo2() {
        let manos = vec![
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];

        let mut partida = EstadoLance::new(40);
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero);
        assert_eq!(partida.tantos(&manos, &Lance::Grande).unwrap(), vec![4, 0]);
        let mut partida = EstadoLance::new(40);
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero);
        assert_eq!(partida.tantos(&manos, &Lance::Chica).unwrap(), vec![4, 0]);
        let mut partida = EstadoLance::new(40);
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero);
        assert_eq!(partida.tantos(&manos, &Lance::Pares).unwrap(), vec![7, 0]);
        let mut partida = EstadoLance::new(40);
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero);
        assert_eq!(partida.tantos(&manos, &Lance::Juego).unwrap(), vec![0, 6]);
    }
    use std::cmp::Ordering::*;

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
