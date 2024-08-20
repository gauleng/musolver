use super::MusError;
use crate::mus::Accion;
use crate::mus::Juego;
use crate::mus::Mano;
use crate::mus::Pares;

use std::cmp;

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
pub enum Lance {
    Grande,
    Chica,
    Pares,
    Punto,
    Juego,
}

impl Lance {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        match self {
            Lance::Grande => a.valor_grande().cmp(&b.valor_grande()),
            Lance::Chica => b.valor_chica().cmp(&a.valor_chica()),
            Lance::Pares => a.pares().cmp(&b.pares()),
            Lance::Juego => a.juego().cmp(&b.juego()),
            Lance::Punto => a.valor_puntos().cmp(&b.valor_puntos()),
        }
    }

    /// Dado un vector de manos de mus, devuelve el índice de la mejor de ellas dado el lance en
    /// juego. Se asume que la primera mano del vector es la del jugador mano y la última la del
    /// postre.
    pub fn mejor_mano(&self, manos: &[Mano]) -> usize {
        let mut indices: Vec<usize> = (0..manos.len()).rev().collect();
        indices.sort_by(|i, j| self.compara_manos(&manos[*i], &manos[*j]));
        *indices.last().unwrap()
    }

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
}

impl EstadoLance {
    pub fn new(apuesta_maxima: u8) -> Self {
        EstadoLance {
            bote: [0, 0],
            activos: [true, true],
            turno: Some(0),
            ultimo_envite: 0,
            apuesta_maxima,
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

    pub fn tantos(&self, manos: &[Mano], lance: &Lance) -> Option<Vec<u8>> {
        if self.turno.is_some() {
            return None;
        }
        let jugadores = [0, 1];
        let activos: Vec<usize> = jugadores.into_iter().filter(|&a| self.activos[a]).collect();
        let se_quisieron = activos.len() > 1;
        let mut apostado = if se_quisieron {
            self.bote[1]
        } else {
            self.bote[0]
        };
        if let Lance::Grande | Lance::Chica = lance {
            if apostado == 0 {
                apostado = 1;
            }
        }

        let ganador = if se_quisieron {
            activos[lance.mejor_mano(manos) % 2]
        } else {
            activos[0]
        };
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
        assert_eq!(partida.tantos(&manos, &Lance::Chica).unwrap(), vec![1, 0]);
        assert_eq!(partida.tantos(&manos, &Lance::Pares).unwrap(), vec![3, 0]);
        assert_eq!(partida.tantos(&manos, &Lance::Juego).unwrap(), vec![0, 2]);
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
