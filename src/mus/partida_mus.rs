use crate::mus::Lance;
use crate::mus::Mano;

use super::MusError;

pub enum Accion {
    Paso,
    Envido(u8),
    Quiero,
    Ordago,
}

pub struct PartidaMus {
    manos: Vec<Mano>,
    bote: [u8; 2],
    activos: Vec<bool>,
    turno: Option<usize>,
    ultimo_envite: usize,
}

impl PartidaMus {
    const MAX_TANTOS: u8 = 40;

    pub fn new(manos: Vec<Mano>) -> Self {
        let m = manos.len();
        PartidaMus {
            manos,
            bote: [0, 0],
            activos: vec![true; m],
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
                if ultimo_bote < PartidaMus::MAX_TANTOS {
                    let nuevo_bote = (ultimo_bote + n).min(PartidaMus::MAX_TANTOS);
                    self.bote[0] = self.bote[1];
                    self.bote[1] = nuevo_bote;
                    self.ultimo_envite = turno;
                }
            }
            Accion::Ordago => {
                if ultimo_bote < PartidaMus::MAX_TANTOS {
                    self.bote[0] = self.bote[1];
                    self.bote[1] = PartidaMus::MAX_TANTOS;
                    self.ultimo_envite = turno;
                }
            }
        }
        self.turno = self.pasar_turno();
        Ok(self.turno)
    }

    fn pasar_turno(&self) -> Option<usize> {
        let turno = self.turno?;
        let num_jugadores = self.activos.len();
        let mut nuevo_turno = turno;
        loop {
            nuevo_turno = (nuevo_turno + 1) % num_jugadores;
            if nuevo_turno == turno || self.ultimo_envite == nuevo_turno {
                return None;
            }
            if self.activos[nuevo_turno] {
                return Some(nuevo_turno);
            }
        }
    }

    pub fn tantos(&self, lance: &Lance) -> Option<Vec<u8>> {
        if self.turno.is_some() {
            return None;
        }
        let jugadores: Vec<usize> = (0..self.manos.len()).collect();
        let activos: Vec<usize> = jugadores.into_iter().filter(|&a| self.activos[a]).collect();
        let apostado = if activos.len() == 1 {
            self.bote[0]
        } else if self.bote[1] == 0 {
            1
        } else {
            self.bote[1]
        };

        let ganador = match activos.len() {
            1 => activos[0],
            _ => {
                let manos_activas = activos.iter().map(|i| self.manos[*i].clone()).collect();
                activos[lance.mejor_mano(&manos_activas)]
            }
        };

        let mut tantos = vec![0; self.manos.len()];
        tantos[ganador] = apostado;
        activos.iter().for_each(|i| match lance {
            Lance::Pares => tantos[*i] += self.manos[*i].num_parejas(),
            Lance::Juego => {
                if self.manos[*i].juego().is_some_and(|v| v == 42) {
                    tantos[*i] += 3
                } else {
                    tantos[*i] += 2
                }
            }
            _ => {}
        });
        tantos[0] = tantos[0] + tantos[2];
        tantos[1] = tantos[1] + tantos[3];
        tantos[2] = tantos[0];
        tantos[3] = tantos[1];

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
        let manos = vec![
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];

        let mut partida = PartidaMus::new(manos);
        assert_eq!(partida.turno(), Some(0));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), Some(1));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), Some(2));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), Some(3));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), None);
    }

    #[test]
    fn test_turno2() {
        let manos = vec![
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];

        let mut partida = PartidaMus::new(manos);
        assert_eq!(partida.actuar(Accion::Envido(2)).unwrap(), Some(1));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), Some(2));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), Some(3));
        assert_eq!(partida.actuar(Accion::Envido(2)).unwrap(), Some(0));
        assert_eq!(partida.actuar(Accion::Envido(2)).unwrap(), Some(3));
        assert_eq!(partida.actuar(Accion::Paso).unwrap(), None);
    }
}
