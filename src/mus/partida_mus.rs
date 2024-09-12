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

#[derive(Debug, Clone)]
pub struct PartidaMus {
    manos: Vec<Mano>,
    //estado_lances: HashMap<Lance, EstadoLance>,
    lances: Vec<(Lance, Option<EstadoLance>)>,
    tantos: [u8; 2],
    lance_actual: Option<usize>,
}

impl PartidaMus {
    const MAX_TANTOS: u8 = 40;

    /// Crea una partida de mus con las manos recibidas como parámetro. Recibe también los tantos
    /// con los que comienzan la partida cada una de las parejas.
    pub fn new(manos: Vec<Mano>, tantos: [u8; 2]) -> Self {
        let mut lances = Vec::with_capacity(4);
        lances.push((Lance::Grande, None));
        lances.push((Lance::Chica, None));
        if Lance::Pares.hay_lance(&manos) {
            lances.push((Lance::Pares, None));
        }
        if Lance::Juego.hay_lance(&manos) {
            lances.push((Lance::Juego, None));
        } else {
            lances.push((Lance::Punto, None));
        }
        let mut p = PartidaMus {
            manos,
            lances,
            lance_actual: Some(0),
            tantos,
        };
        let e = p.crear_estado_lance(Lance::Grande);
        let _ = p.lances[0].1.insert(e);
        p
    }

    /// Crea una partida de mus en la que solo se juega un lance con la manos recibidas como
    /// parámetro. Recibe también los tantos con los que comienzan la partida cada una de las
    /// parejas.
    ///
    /// La partida solo se crea si se juega el lance. En caso contrario devuelve None.
    /// Esto puede ocurrir por ejemplo si se desea crear una partida para el lance de pares
    /// con cuatro manos sin jugadas de pares, o que solo una de las parejas tiene pares.
    pub fn new_partida_lance(lance: Lance, manos: Vec<Mano>, tantos: [u8; 2]) -> Option<Self> {
        let lances = vec![(lance, None)];
        if lance.se_juega(&manos) {
            let mut p = Self {
                manos,
                lances,
                lance_actual: Some(0),
                tantos,
            };
            let e = p.crear_estado_lance(lance);
            let _ = p.lances[0].1.insert(e);
            Some(p)
        } else {
            None
        }
    }

    fn crear_estado_lance(&self, l: Lance) -> EstadoLance {
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
        e
    }

    fn tanteo_final_lance(&mut self, l: &Lance, e: &mut EstadoLance) {
        let g = e.ganador().unwrap_or_else(|| {
            let g = e.resolver_lance(&self.manos, l);
            if let Apuesta::Tantos(t) = e.tantos_apostados() {
                self.tantos[g] += t;
            }
            g
        });
        self.anotar_tantos(
            g,
            l.tantos_mano(&self.manos[g]) + l.tantos_mano(&self.manos[g + 2]) + l.bonus(),
        );
    }

    fn tanteo_lance(&mut self, lance: &Lance, estado_lance: &mut EstadoLance) {
        let apuesta = estado_lance.tantos_apostados();
        if let Apuesta::Ordago = apuesta {
            estado_lance.resolver_lance(&self.manos, lance);
        }
        let ganador = estado_lance.ganador();
        if let Some(g) = ganador {
            match apuesta {
                Apuesta::Tantos(t) => self.anotar_tantos(g, t),
                Apuesta::Ordago => self.anotar_tantos(g, Self::MAX_TANTOS),
            }
        }
    }

    fn siguiente_lance(&mut self) -> Option<&EstadoLance> {
        let mut lance_actual = self.lance_actual?;
        if lance_actual < self.lances.len() - 1 {
            lance_actual += 1;
            self.lance_actual = Some(lance_actual);
            let lance = self.lances[lance_actual].0;
            let estado_lance = self.crear_estado_lance(lance);
            let _ = self.lances[lance_actual].1.insert(estado_lance);
            self.lances[lance_actual].1.as_ref()
        } else {
            let lances = std::mem::take(&mut self.lances);
            lances
                .into_iter()
                .for_each(|l| self.tanteo_final_lance(&l.0, &mut l.1.unwrap()));
            self.lance_actual = None;
            None
        }
    }

    /// Realiza la acción recibida como parámetro. Devuelve el turno de la siguiente pareja o Ok(None)
    /// si la partida ha terminado. Esta función devuelve error si se llama tras haber acabado la
    /// partida.
    pub fn actuar(&mut self, accion: Accion) -> Result<Option<usize>, MusError> {
        let lance_actual = self.lance_actual.ok_or(MusError::AccionNoValida)?;
        let lance = self.lances[lance_actual].0;
        let mut estado_lance = self.lances[lance_actual].1.take().unwrap();
        let a = estado_lance.actuar(accion);
        if let Ok(None) = a {
            self.tanteo_lance(&lance, &mut estado_lance);
            let _ = self.lances[lance_actual].1.insert(estado_lance);
            loop {
                let estado_lance = self.siguiente_lance();
                if let Some(e) = estado_lance {
                    if e.turno().is_some() {
                        return Ok(e.turno());
                    }
                } else {
                    return Ok(None);
                }
            }
        } else {
            let _ = self.lances[lance_actual].1.insert(estado_lance);
            a
        }
    }

    /// Devuelve el turno de la pareja a la que le toca jugar.
    pub fn turno(&self) -> Option<usize> {
        let lance_actual = self.lance_actual?;
        let estado_lance = self.lances[lance_actual].1.as_ref().unwrap();
        estado_lance.turno()
    }

    /// Devuelve los tantos que lleva cada pareja.
    pub fn tantos(&self) -> &[u8] {
        &self.tantos
    }

    fn anotar_tantos(&mut self, pareja: usize, tantos: u8) {
        self.tantos[pareja] += tantos;
        if self.tantos[pareja] >= Self::MAX_TANTOS {
            self.tantos[pareja] = Self::MAX_TANTOS;
            self.tantos[1 - pareja] = 0;
            self.lance_actual = None;
        }
    }

    pub fn lance_actual(&self) -> Option<Lance> {
        self.lance_actual.map(|v| self.lances[v].0)
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

        let mut partida = PartidaMus::new(manos, [0, 0]);
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

        let mut partida = PartidaMus::new(manos, [0, 0]);
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
        let mut partida = PartidaMus::new(manos, [0, 0]);
        let _ = partida.actuar(Accion::Ordago);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(), vec![1, 0]);
        let _ = partida.actuar(Accion::Ordago);
        let _ = partida.actuar(Accion::Quiero);
        assert_eq!(partida.tantos(), vec![40, 0]);
        assert_eq!(partida.turno(), None);
    }

    #[test]
    fn test_partida_lance() {
        let manos = vec![
            Mano::try_from("CC76").unwrap(),
            Mano::try_from("CCC1").unwrap(),
            Mano::try_from("1111").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];
        let mut partida_lance = PartidaMus::new_partida_lance(Lance::Juego, manos, [0, 0]);
        assert!(partida_lance.is_some());
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        assert_eq!(partida_lance.as_ref().unwrap().lance_actual(), None);
        assert_eq!(partida_lance.as_ref().unwrap().tantos(), [0, 3]);
        let manos = vec![
            Mano::try_from("257C").unwrap(),
            Mano::try_from("CC76").unwrap(),
            Mano::try_from("CCC1").unwrap(),
            Mano::try_from("1111").unwrap(),
        ];
        let mut partida_lance = PartidaMus::new_partida_lance(Lance::Juego, manos, [0, 0]);
        assert_eq!(partida_lance.as_ref().unwrap().turno(), Some(1));
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        assert_eq!(partida_lance.as_ref().unwrap().tantos(), [3, 0]);
    }
}
