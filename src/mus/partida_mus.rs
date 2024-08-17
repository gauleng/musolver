use crate::mus::Lance;
use crate::mus::Mano;

pub enum Accion {
    Paso,
    Envido(u8),
    Quiero,
    Ordago,
}

pub struct PartidaMus {
    manos: Vec<Mano>,
    bote: Vec<u8>,
    activos: Vec<bool>,
    turno: Option<usize>,
    ultimo_envite: Option<usize>,
}

impl PartidaMus {
    pub fn new(manos: Vec<Mano>) -> Self {
        let m = manos.len();
        PartidaMus {
            manos,
            bote: vec![0],
            activos: vec![true; m],
            turno: Some(0),
            ultimo_envite: None,
        }
    }

    pub fn actuar(&mut self, a: Accion) {
        let turno = match self.turno {
            None => return,
            Some(t) => t,
        };
        let ultimo_bote = self.bote.last().unwrap();
        match a {
            Accion::Paso => {
                self.activos[turno] = false;
            }
            Accion::Quiero => {}
            Accion::Envido(n) => {
                if *ultimo_bote < 40 {
                    let nuevo_bote = ultimo_bote + n;
                    self.bote.push(nuevo_bote.min(40));
                    self.ultimo_envite = Some(turno);
                }
            }
            Accion::Ordago => {
                if *ultimo_bote < 40 {
                    self.bote.push(40);
                    self.ultimo_envite = Some(turno);
                }
            }
        }
        self.turno = self.pasar_turno();
    }

    fn pasar_turno(&mut self) -> Option<usize> {
        let turno = match self.turno {
            None => return None,
            Some(t) => t,
        };
        let num_jugadores = self.activos.len();
        let mut nuevo_turno = turno;
        loop {
            nuevo_turno = (nuevo_turno + 1) % num_jugadores;
            if nuevo_turno == turno
                || (self.ultimo_envite.is_some() && nuevo_turno == self.ultimo_envite.unwrap())
            {
                return None;
            }
            if self.activos[turno] {
                return Some(nuevo_turno);
            }
        }
    }

    pub fn tantos(&self, lance: &dyn Lance) -> Option<Vec<u8>> {
        if self.turno.is_some() {
            return None;
        }
        let jugadores: Vec<usize> = (0..self.manos.len()).collect();
        let activos: Vec<usize> = jugadores.into_iter().filter(|&a| self.activos[a]).collect();
        let apostado = match activos.len() {
            0 => 1,
            1 => *self.bote.get(self.bote.len() - 2).unwrap(),
            _ => *self.bote.last().unwrap(),
        };

        let ganador = match activos.len() {
            0 => lance.mejor_mano(&self.manos),
            1 => activos[0],
            _ => {
                let manos_activas = activos.iter().map(|i| self.manos[*i].clone()).collect();
                activos[lance.mejor_mano(&manos_activas)]
            }
        };
        let pareja = ganador % 2;

        let mut tantos = vec![0; self.manos.len()];
        tantos[pareja] = apostado;
        tantos[2 + pareja] = apostado;
        Some(tantos)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn name() {}
}
