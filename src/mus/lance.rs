use super::MusError;
use crate::mus::Accion;
use crate::mus::Mano;

use std::cmp;

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
pub enum Juego {
    Resto(u8),
    Treintaydos,
    Treintayuna,
}

#[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
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

    pub fn apuesta_minima(&self) -> u8 {
        if let Lance::Grande | Lance::Chica = self {
            1
        } else {
            0
        }
    }

    pub fn bonus(&self) -> u8 {
        if let Lance::Punto = self {
            1
        } else {
            0
        }
    }

    fn se_juega_jugadas<T>(&self, manos: &[Option<T>]) -> bool
    where
        T: Copy + Clone,
    {
        (manos[0].or_else(|| manos[2]))
            .and_then(|_| manos[1].or_else(|| manos[3]))
            .is_some()
    }

    fn hay_lance_jugadas<T>(&self, manos: &[Option<T>]) -> bool {
        manos.iter().any(|j| j.is_some())
    }

    fn turno_inicial_jugadas<T>(&self, manos: &[Option<T>]) -> usize {
        let pares_filt = manos.iter().filter(|p| p.is_some()).count();
        if pares_filt == 2 || pares_filt == 3 {
            if manos[1].is_some() && manos[2].is_some() && manos[3].is_none() {
                1
            } else {
                0
            }
        } else {
            0
        }
    }

    fn jugadas<T, F>(&self, manos: &[Mano], f: F) -> Vec<Option<T>>
    where
        F: Fn(&Mano) -> Option<T>,
    {
        manos.iter().map(f).collect()
    }

    pub fn turno_inicial(&self, manos: &[Mano]) -> usize {
        match self {
            Lance::Grande | Lance::Chica | Lance::Punto => 0,
            Lance::Pares => self.turno_inicial_jugadas(&self.jugadas(manos, |m| m.pares())),
            Lance::Juego => self.turno_inicial_jugadas(&self.jugadas(manos, |m| m.juego())),
        }
    }

    pub fn hay_lance(&self, manos: &[Mano]) -> bool {
        match self {
            Lance::Grande | Lance::Chica => true,
            Lance::Pares => self.hay_lance_jugadas(&self.jugadas(manos, |m| m.pares())),
            Lance::Juego => self.hay_lance_jugadas(&self.jugadas(manos, |m| m.juego())),
            Lance::Punto => !self.hay_lance_jugadas(&self.jugadas(manos, |m| m.juego())),
        }
    }

    pub fn se_juega(&self, manos: &[Mano]) -> bool {
        match self {
            Lance::Grande | Lance::Chica => true,
            Lance::Pares => self.se_juega_jugadas(&self.jugadas(manos, |m| m.pares())),
            Lance::Juego => self.se_juega_jugadas(&self.jugadas(manos, |m| m.juego())),
            Lance::Punto => !self.hay_lance_jugadas(&self.jugadas(manos, |m| m.juego())),
        }
    }
}

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
}

impl EstadoLance {
    pub fn new(apuesta_minima: u8, apuesta_maxima: u8, turno_inicial: usize) -> Self {
        EstadoLance {
            bote: [Apuesta::Tantos(0), Apuesta::Tantos(0)],
            activos: [true, true],
            turno: Some(turno_inicial),
            ultimo_envite: 0,
            apuesta_minima,
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
    pub fn resolver_lance<R>(&mut self, manos: &[Mano], r: &R) -> usize
    where
        R: RankingManos,
    {
        self.turno = None;
        *self.ganador.get_or_insert_with(|| r.mejor_mano(manos) % 2)
    }

    /// India si ya hay un ganador en el lance, bien sea porque una pareja ha rechazado un envite o
    /// porque el lance ya está resuelto. En caso de que todavía un no haya ganador, esta función
    /// devuelve None.
    pub fn ganador(&self) -> Option<usize> {
        self.ganador
    }

    pub fn tantos(&mut self, manos: &[Mano], lance: &Lance) -> Option<Vec<u8>> {
        if self.turno.is_some() {
            return None;
        }
        let apostado = self.tantos_apostados();
        let ganador = self.resolver_lance(manos, lance);
        let manos_ganadoras = [ganador, ganador + 2];

        let mut tantos = vec![0, 0];
        match apostado {
            Apuesta::Tantos(t) => {
                tantos[ganador] = t;
                tantos[ganador] += lance.tantos_mano(&manos[manos_ganadoras[0]])
                    + lance.tantos_mano(&manos[manos_ganadoras[1]]);
                if let Lance::Punto = lance {
                    tantos[ganador] += 1;
                }
            }
            Apuesta::Ordago => tantos[ganador] = 40,
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
        let mut partida = EstadoLance::new(0, 40, 0);
        assert_eq!(partida.turno(), Some(0));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), Some(1));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), None);
    }

    #[test]
    fn test_turno2() {
        let mut partida = EstadoLance::new(0, 40, 0);
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
        let mut partida = EstadoLance::new(1, 40, 0);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance(&manos, &Lance::Grande);
        assert_eq!(partida.ganador(), Some(0));

        let manos = vec![
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RRRR").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance(&manos, &Lance::Grande);
        assert_eq!(partida.ganador(), Some(0));

        let manos = vec![
            Mano::try_from("RRRC").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance(&manos, &Lance::Grande);
        assert_eq!(partida.ganador(), Some(1));

        let manos = vec![
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("1111").unwrap(),
            Mano::try_from("1111").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance(&manos, &Lance::Grande);
        assert_eq!(partida.ganador(), Some(0));

        let manos = vec![
            Mano::try_from("1111").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("1111").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance(&manos, &Lance::Grande);
        assert_eq!(partida.ganador(), Some(1));

        let manos = vec![
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RR11").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("R111").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance(&manos, &Lance::Grande);
        assert_eq!(partida.ganador(), Some(1));

        let manos = vec![
            Mano::try_from("R111").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RR11").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance(&manos, &Lance::Grande);
        assert_eq!(partida.ganador(), Some(1));

        let manos = vec![
            Mano::try_from("R111").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RR11").unwrap(),
            Mano::try_from("R111").unwrap(),
        ];
        let mut partida = EstadoLance::new(1, 40, 0);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        partida.resolver_lance(&manos, &Lance::Grande);
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
        let mut e = EstadoLance::new(0, 40, 0);
        let _ = e.actuar(Accion::Paso);
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), None);
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(0));

        let mut e = EstadoLance::new(1, 40, 0);
        let _ = e.actuar(Accion::Paso);
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), None);
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(1));

        let mut e = EstadoLance::new(0, 40, 0);
        let _ = e.actuar(Accion::Envido(2));
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), Some(0));
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(1));

        let mut e = EstadoLance::new(1, 40, 0);
        let _ = e.actuar(Accion::Envido(2));
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), Some(0));
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(1));

        let mut e = EstadoLance::new(0, 40, 0);
        let _ = e.actuar(Accion::Paso);
        let _ = e.actuar(Accion::Envido(2));
        let _ = e.actuar(Accion::Paso);
        assert_eq!(e.ganador(), Some(1));
        assert_eq!(e.tantos_apostados(), Apuesta::Tantos(1));

        let mut e = EstadoLance::new(1, 40, 0);
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
