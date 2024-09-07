use std::collections::HashMap;
use std::fmt::Display;

use crate::mus::Lance;
use crate::mus::Mano;

use super::Apuesta;
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
    lance_actual: Option<usize>,
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
            lance_actual: Some(0),
            estado_lances,
            tantos: [0, 0],
        }
    }

    pub fn siguiente_lance(&mut self) -> Option<Lance> {
        let mut lance_actual = self.lance_actual?;
        if lance_actual < self.lances.len() - 1 {
            lance_actual += 1;
            self.lance_actual = Some(lance_actual);
            let l = self.lances[lance_actual];
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
                let g = e.ganador().unwrap_or_else(|| {
                    let g = e.resolver_lance(&self.manos, l);
                    if let Apuesta::Tantos(t) = e.tantos_apostados() {
                        self.tantos[g] += t;
                    }
                    g
                });
                self.tantos[g] += l.tantos_mano(&self.manos[g]) + l.tantos_mano(&self.manos[g + 2]);
                self.tantos[g] += l.bonus();
            });
            self.lance_actual = None;
            None
        }
    }

    pub fn actuar(&mut self, accion: Accion) -> Result<Option<usize>, MusError> {
        let lance_actual = self.lance_actual.ok_or(MusError::AccionNoValida)?;
        let lance = self.lances[lance_actual];
        let estado_lance = self.estado_lances.get_mut(&lance).unwrap();
        let a = estado_lance.actuar(accion);
        if let Ok(None) = a {
            let apuesta = estado_lance.tantos_apostados();
            if let Apuesta::Ordago = apuesta {
                estado_lance.resolver_lance(&self.manos, &lance);
            }
            let ganador = estado_lance.ganador();
            if let Some(g) = ganador {
                match apuesta {
                    Apuesta::Tantos(t) => self.anotar_tantos(g, t),
                    Apuesta::Ordago => self.anotar_tantos(g, 40),
                }
            }
            loop {
                let lance = self.siguiente_lance();
                if let Some(l) = lance {
                    if l.se_juega(&self.manos) {
                        let e = self.estado_lances.get(&l).unwrap();
                        return Ok(e.turno());
                    }
                } else {
                    return Ok(None);
                }
            }
        } else {
            a
        }
    }

    pub fn turno(&self) -> Option<usize> {
        let lance_actual = self.lance_actual?;
        let estado_lance = self.estado_lances.get(&self.lances[lance_actual]).unwrap();
        estado_lance.turno()
    }

    pub fn tantos(&self) -> &[u8] {
        &self.tantos
    }

    fn anotar_tantos(&mut self, pareja: usize, tantos: u8) {
        self.tantos[pareja] += tantos;
        if self.tantos[pareja] >= 40 {
            self.tantos[pareja] = 40;
            self.tantos[1 - pareja] = 0;
            self.lance_actual = None;
        }
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
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
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
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero);
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero);
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero);
        assert_eq!(partida.tantos(), vec![11, 8]);
    }

    #[test]
    fn test_ordago() {
        let manos = vec![
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];
        let mut partida = PartidaMus::new(manos);
        let _ = partida.actuar(Accion::Ordago);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(), vec![1, 0]);
        let _ = partida.actuar(Accion::Ordago);
        let _ = partida.actuar(Accion::Quiero);
        assert_eq!(partida.tantos(), vec![40, 0]);
        assert_eq!(partida.turno(), None);
    }
}
