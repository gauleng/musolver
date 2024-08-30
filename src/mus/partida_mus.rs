use std::collections::HashMap;
use std::fmt::Display;

use crate::mus::Lance;
use crate::mus::Mano;

use super::EstadoLance;
use super::MusError;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum Accion {
    Paso,
    Envido(u8),
    Quiero,
    Ordago,
}

impl Display for Accion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Accion::Paso => f.write_str("p"),
            Accion::Envido(n) => f.write_fmt(format_args!("e{}", n)),
            Accion::Quiero => f.write_str("q"),
            Accion::Ordago => f.write_str("o"),
        }
    }
}

pub struct PartidaMus {
    manos: Vec<Mano>,
    estado_lances: HashMap<Lance, EstadoLance>,
    lances: Vec<Lance>,
    tantos: [u8; 2],
    lance_actual: usize,
}

impl PartidaMus {
    const MAX_TANTOS: u8 = 40;

    pub fn new(manos: Vec<Mano>) -> Self {
        let mut lances = Vec::with_capacity(4);
        lances.push(Lance::Grande);
        lances.push(Lance::Chica);
        if Lance::Pares.hay_lance(&manos) {
            lances.push(Lance::Pares);
        }
        if Lance::Juego.hay_lance(&manos) {
            lances.push(Lance::Juego);
        } else {
            lances.push(Lance::Punto);
        }
        let estado_lances = HashMap::from([(Lance::Grande, EstadoLance::new(1, 40, 0))]);
        PartidaMus {
            manos,
            lances,
            lance_actual: 0,
            estado_lances,
            tantos: [0, 0],
        }
    }

    pub fn siguiente_lance(&mut self) -> Option<Lance> {
        if self.lance_actual < self.lances.len() - 1 {
            self.lance_actual += 1;
            let l = self.lances[self.lance_actual];
            let tantos_restantes = [
                Self::MAX_TANTOS - self.tantos[0],
                Self::MAX_TANTOS - self.tantos[1],
            ];
            let mut e = EstadoLance::new(
                l.apuesta_minima(),
                tantos_restantes[0].max(tantos_restantes[1]),
                l.turno_inicial(&self.manos),
            );
            if !l.se_juega(&self.manos) {
                e.resolver_lance(&self.manos, &l);
            }
            self.estado_lances.insert(l, e);
            Some(l)
        } else {
            self.lances.iter().for_each(|l| {
                let e = self.estado_lances.get_mut(l).unwrap();
                let g = e.ganador().map_or_else(
                    || {
                        let g = e.resolver_lance(&self.manos, l);
                        self.tantos[g] += e.tantos_apostados();
                        g
                    },
                    |v| v,
                );
                self.tantos[g] += l.tantos_mano(&self.manos[g]) + l.tantos_mano(&self.manos[g + 2]);
                self.tantos[g] += l.bonus();
            });
            None
        }
    }

    pub fn actuar(&mut self, accion: Accion) -> Result<Option<usize>, MusError> {
        if self.lance_actual >= self.lances.len() {
            return Err(MusError::AccionNoValida);
        }
        let estado_lance = self
            .estado_lances
            .get_mut(&self.lances[self.lance_actual])
            .unwrap();
        let a = estado_lance.actuar(accion);
        if let Ok(None) = a {
            let g = estado_lance.ganador();
            if g.is_some() {
                self.tantos[g.unwrap()] += estado_lance.tantos_apostados();
            }
        }
        a
    }

    pub fn turno(&self) -> Option<usize> {
        if self.lance_actual >= self.lances.len() {
            None
        } else {
            let estado_lance = self
                .estado_lances
                .get(&self.lances[self.lance_actual])
                .unwrap();
            estado_lance.turno()
        }
    }

    pub fn tantos(&self) -> &[u8] {
        &self.tantos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tanteo() {
        let manos = vec![
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];

        let mut partida = PartidaMus::new(manos);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.siguiente_lance(), Some(Lance::Chica));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.siguiente_lance(), Some(Lance::Pares));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.siguiente_lance(), Some(Lance::Juego));
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.siguiente_lance(), None);
        assert_eq!(partida.tantos(), vec![5, 2]);
    }

    #[test]
    fn test_tanteo2() {
        let manos = vec![
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];

        let mut partida = PartidaMus::new(manos);
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(), vec![0, 2]);
        partida.siguiente_lance();
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero);
        partida.siguiente_lance();
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero);
        partida.siguiente_lance();
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero);
        assert_eq!(partida.siguiente_lance(), None);
        assert_eq!(partida.tantos(), vec![11, 8]);
    }
}
