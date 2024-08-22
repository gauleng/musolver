use std::collections::HashMap;

use crate::mus::Lance;
use crate::mus::Mano;

use super::EstadoLance;
use super::MusError;

pub enum Accion {
    Paso,
    Envido(u8),
    Quiero,
    Ordago,
}

pub struct PartidaMus {
    manos: Vec<Mano>,
    lances: HashMap<Lance, EstadoLance>,
    tantos: [u8; 2],
    lance_actual: Lance,
}

impl PartidaMus {
    const MAX_TANTOS: u8 = 40;

    pub fn new(manos: Vec<Mano>) -> Self {
        PartidaMus {
            manos,
            lances: HashMap::from([(Lance::Grande, EstadoLance::new(1, 40))]),
            lance_actual: Lance::Grande,
            tantos: [0, 0],
        }
    }

    fn siguiente_lance(&self) -> Option<Lance> {
        match self.lance_actual {
            Lance::Grande => Some(Lance::Chica),
            Lance::Chica => Some(Lance::Pares),
            Lance::Pares => {
                let hay_juego = self.manos.iter().map(|m| m.juego()).any(|j| j.is_some());
                if hay_juego {
                    Some(Lance::Juego)
                } else {
                    Some(Lance::Punto)
                }
            }
            Lance::Juego | Lance::Punto => None,
        }
    }

    pub fn actuar(&mut self, accion: Accion) -> Result<Option<usize>, MusError> {
        let estado_lance = self.lances.get_mut(&self.lance_actual);
        match estado_lance.unwrap().actuar(accion) {
            Ok(None) => {
                let siguiente_lance = self.siguiente_lance();
                if siguiente_lance.is_some() {
                    let nuevo_lance = EstadoLance::new(0, 40);
                    let turno = nuevo_lance.turno();
                    self.lances.insert(siguiente_lance.unwrap(), nuevo_lance);
                    self.lance_actual = siguiente_lance.unwrap();
                    Ok(turno)
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(e),
            t => t,
        }
    }
}
