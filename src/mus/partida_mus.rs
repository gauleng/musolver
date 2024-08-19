use crate::mus::Lance;
use crate::mus::Mano;

use super::MusError;

pub enum Accion {
    Paso,
    Envido(u8),
    Quiero,
    Ordago,
}

pub struct EstadoLance {
    bote: [u8; 2],
    activos: [bool; 2],
    turno: Option<usize>,
    ultimo_envite: usize,
}

impl EstadoLance {
    const MAX_TANTOS: u8 = 40;

    pub fn new() -> Self {
        EstadoLance {
            bote: [0, 0],
            activos: [true, true],
            turno: Some(0),
            ultimo_envite: 0,
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
                if ultimo_bote < EstadoLance::MAX_TANTOS {
                    let nuevo_bote = (ultimo_bote + n.max(2)).min(EstadoLance::MAX_TANTOS);
                    self.bote[0] = self.bote[1];
                    self.bote[1] = nuevo_bote;
                    self.ultimo_envite = turno;
                }
            }
            Accion::Ordago => {
                if ultimo_bote < EstadoLance::MAX_TANTOS {
                    self.bote[0] = self.bote[1];
                    self.bote[1] = EstadoLance::MAX_TANTOS;
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
        return None;
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

    pub fn tantos(&self, manos: &Vec<Mano>, lance: &Lance) -> Option<Vec<u8>> {
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
        if apostado == 0 {
            apostado = 1;
        }

        let ganador = if se_quisieron {
            activos[lance.mejor_mano(&manos) % 2]
        } else {
            activos[0]
        };
        let manos_ganadoras = [ganador, ganador + 2];

        let mut tantos = vec![0, 0];
        tantos[ganador] = apostado;
        match lance {
            Lance::Pares => {
                tantos[ganador] += manos[manos_ganadoras[0]].num_parejas()
                    + manos[manos_ganadoras[1]].num_parejas();
            }
            Lance::Juego => {
                if let Some(v) = manos[manos_ganadoras[0]].valor_juego() {
                    if v == 42 {
                        tantos[ganador] += 3
                    } else {
                        tantos[ganador] += 2
                    }
                }
                if let Some(v) = manos[manos_ganadoras[1]].valor_juego() {
                    if v == 42 {
                        tantos[ganador] += 3
                    } else {
                        tantos[ganador] += 2
                    }
                }
            }
            _ => {}
        };
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
        let mut partida = EstadoLance::new();
        assert_eq!(partida.turno(), Some(0));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), Some(1));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), None);
    }

    #[test]
    fn test_turno2() {
        let mut partida = EstadoLance::new();
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

        let mut partida = EstadoLance::new();
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);

        assert_eq!(partida.tantos(&manos, &Lance::Grande).unwrap(), vec![1, 0]);
        assert_eq!(partida.tantos(&manos, &Lance::Chica).unwrap(), vec![1, 0]);
        assert_eq!(partida.tantos(&manos, &Lance::Pares).unwrap(), vec![3, 0]);
        assert_eq!(partida.tantos(&manos, &Lance::Juego).unwrap(), vec![0, 2]);
    }
}
