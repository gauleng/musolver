use std::fmt::Display;

use serde::Deserialize;
use serde::Serialize;

use crate::mus::Lance;
use crate::mus::Mano;

use super::Apuesta;
use super::EstadoLance;
use super::MusError;
use super::Turno;

/// Acciones posibles durante una partida de mus.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Accion {
    Paso,
    Quiero,
    Envido(u8),
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
struct ResultadoLance {
    ganador: u8,
    tantos: u8,
}

#[derive(Debug, Clone)]
pub struct PartidaMus {
    manos: [Mano; 4],
    lances: Vec<(Lance, Option<ResultadoLance>)>,
    tantos: [u8; 2],
    idx_lance: usize,
    estado_lance: Option<EstadoLance>,
}

impl PartidaMus {
    pub const MAX_TANTOS: u8 = 40;

    /// Crea una partida de mus con las manos recibidas como parámetro. Las manos deben estar en un
    /// array y se asume que la primera posición se corresponde con la mano del jugador mano y la
    /// última con la del jugador postre.
    ///
    /// Recibe también los tantos con los que comienzan la partida
    /// cada una de las parejas.
    pub fn new(manos: [Mano; 4], tantos: [u8; 2]) -> Self {
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
            idx_lance: 0,
            tantos,
            estado_lance: None,
        };
        let e = p.crear_estado_lance(Lance::Grande);
        p.estado_lance = Some(e);
        p
    }

    /// Crea una partida de mus en la que solo se juega un lance con la manos recibidas como
    /// parámetro. Recibe también los tantos con los que comienzan la partida cada una de las
    /// parejas.
    ///
    /// La partida solo se crea si se juega el lance. En caso contrario devuelve None.
    /// Esto puede ocurrir por ejemplo si se desea crear una partida para el lance de pares
    /// con cuatro manos sin jugadas de pares, o que solo una de las parejas tiene pares.
    pub fn new_partida_lance(lance: Lance, manos: [Mano; 4], tantos: [u8; 2]) -> Option<Self> {
        let lances = vec![(lance, None)];
        let mut p = Self {
            manos,
            lances,
            idx_lance: 0,
            tantos,
            estado_lance: None,
        };
        let e = p.crear_estado_lance(lance);
        e.turno()?;
        p.estado_lance = Some(e);
        Some(p)
    }

    fn crear_estado_lance(&self, l: Lance) -> EstadoLance {
        let tantos_restantes = [
            Self::MAX_TANTOS - self.tantos[0],
            Self::MAX_TANTOS - self.tantos[1],
        ];
        let mut e = EstadoLance::new(
            &l,
            &self.manos,
            tantos_restantes[0].max(tantos_restantes[1]),
        );
        if !l.se_juega(&self.manos) {
            e.resolver_lance();
        }
        e
    }

    fn tanteo_final_lance(&mut self, l: &Lance) {
        if let Some(estado_lance) = &mut self.estado_lance {
            let mut tantos = 0;
            let ganador = estado_lance.ganador().unwrap_or_else(|| {
                let g = estado_lance.resolver_lance();
                if let Apuesta::Tantos(t) = estado_lance.tantos_apostados() {
                    tantos += t;
                }
                g
            });

            tantos += l.tantos_mano(&self.manos[ganador as usize])
                + l.tantos_mano(&self.manos[ganador as usize + 2])
                + l.bonus();
            self.lances[self.idx_lance].1 = Some(ResultadoLance { ganador, tantos });
        }
    }

    fn tanteo_envites_lance(&mut self) {
        if let Some(estado_lance) = &mut self.estado_lance {
            let apuesta = estado_lance.tantos_apostados();
            if let Apuesta::Ordago = apuesta {
                estado_lance.resolver_lance();
            }
            let ganador = estado_lance.ganador();
            if let Some(g) = ganador {
                match apuesta {
                    Apuesta::Tantos(t) => self.anotar_tantos(g, t),
                    Apuesta::Ordago => self.anotar_tantos(g, Self::MAX_TANTOS),
                }
            }
        }
    }

    fn tanteo_final(&mut self) {
        let lances = std::mem::take(&mut self.lances);
        for l in lances {
            if let Some(r) = l.1 {
                self.anotar_tantos(r.ganador, r.tantos);
                if self.tantos[0] == Self::MAX_TANTOS || self.tantos[1] == Self::MAX_TANTOS {
                    break;
                }
            }
        }
    }

    fn siguiente_lance(&mut self) -> Option<&EstadoLance> {
        self.estado_lance.as_ref()?;
        if self.idx_lance < self.lances.len() - 1 {
            self.idx_lance += 1;
            let lance = self.lances[self.idx_lance].0;
            let estado_lance = self.crear_estado_lance(lance);
            self.estado_lance = Some(estado_lance);
        } else {
            self.estado_lance = None;
        }
        self.estado_lance.as_ref()
    }

    /// Realiza la acción recibida como parámetro. Devuelve el turno de la siguiente pareja o Ok(None)
    /// si la partida ha terminado. Esta función devuelve error si se llama tras haber acabado la
    /// partida.
    pub fn actuar(&mut self, accion: Accion) -> Result<Option<Turno>, MusError> {
        let estado_lance = self.estado_lance.as_mut().ok_or(MusError::AccionNoValida)?;
        let turno = estado_lance.actuar(accion)?;
        if turno.is_some() {
            return Ok(turno);
        }
        let lance = self.lances[self.idx_lance].0;
        self.tanteo_envites_lance();
        self.tanteo_final_lance(&lance);
        while let Some(e) = self.siguiente_lance() {
            if e.turno().is_some() {
                return Ok(e.turno());
            }
        }
        self.tanteo_final();
        Ok(None)
    }

    /// Devuelve el turno de la pareja a la que le toca jugar.
    pub fn turno(&self) -> Option<Turno> {
        let estado_lance = self.estado_lance.as_ref()?;
        estado_lance.turno()
    }

    /// Devuelve los tantos que lleva cada pareja.
    pub fn tantos(&self) -> &[u8; 2] {
        &self.tantos
    }

    fn anotar_tantos(&mut self, pareja: u8, tantos: u8) {
        let pareja = pareja as usize;
        self.tantos[pareja] += tantos;
        if self.tantos[pareja] >= Self::MAX_TANTOS {
            self.tantos[pareja] = Self::MAX_TANTOS;
            self.tantos[1 - pareja] = 0;
            self.estado_lance = None;
        }
    }

    /// Devuelve el lance en curso, o None si la partida ya ha acabado.
    pub fn lance_actual(&self) -> Option<Lance> {
        self.estado_lance
            .as_ref()
            .map(|_| self.lances[self.idx_lance].0)
    }

    /// Devuelve las manos de los jugadores.
    pub fn manos(&self) -> &[Mano; 4] {
        &self.manos
    }

    /// Indica si hubo algún envite en el lance en curso. En caso de que la partida esté
    /// finalizada, devuelve false.
    pub fn hay_envites(&self) -> bool {
        self.estado_lance.as_ref().map_or_else(
            || false,
            |estado_lance| estado_lance.ultima_apuesta() > Apuesta::Tantos(0),
        )
    }

    /// Devuelve hasta cuántos tantos se ha elevado la apuesta del lance actual. Se incluye en este
    /// valor los envites que todavía no han sido aceptados por la pareja rival.
    pub fn ultima_apuesta(&self) -> Apuesta {
        self.estado_lance.as_ref().map_or_else(
            || Apuesta::Tantos(0),
            |estado_lance| estado_lance.ultima_apuesta(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tanteo() {
        let manos = [
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];

        let mut partida = PartidaMus::new(manos, [0, 0]);
        for _ in 0..16 {
            let _ = partida.actuar(Accion::Paso);
        }
        assert_eq!(partida.tantos(), &[5, 2]);
    }

    #[test]
    fn test_tanteo2() {
        let manos = [
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];

        let mut partida = PartidaMus::new(manos, [0, 0]);
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0 (0)
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0 (2)
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 1 (1)
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 1 (3)
        let _ = partida.actuar(Accion::Paso); // Pareja 0 (0)
        let _ = partida.actuar(Accion::Paso); // Pareja 0 (2)
        assert_eq!(partida.tantos(), &[0, 2]);

        assert_eq!(partida.lance_actual(), Some(Lance::Chica));
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0 (0)
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0 (2)
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 1
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero); // Pareja 0
        let _ = partida.actuar(Accion::Quiero); // 4, 2
        assert_eq!(partida.tantos(), &[0, 2]);

        // 3 no tiene pares, entonces "juega primero" la pareja 1
        assert_eq!(partida.lance_actual(), Some(Lance::Pares));
        let _ = partida.actuar(Accion::Envido(2)); // Jugador 1
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Paso); // 6, 2
        assert_eq!(partida.tantos(), &[2, 2]);

        // Tienen juego 2 y 3. Entonces, "juega primero" la pareja 1
        assert_eq!(partida.lance_actual(), Some(Lance::Juego));
        let _ = partida.actuar(Accion::Envido(2)); // Jugador 1
        let _ = partida.actuar(Accion::Envido(2)); // Jugador 0
        let _ = partida.actuar(Accion::Quiero); // Jugador 1
        assert_eq!(partida.tantos(), &[9, 8]);

        /*
        Pareja 0
            4 chica
            1 par
            2 medias
        Pareja 1
            4 envite juego
            2 juego
         */
    }

    #[test]
    fn test_punto_miedo() {
        let manos = [
            Mano::try_from("1134").unwrap(),
            Mano::try_from("571S").unwrap(),
            Mano::try_from("3544").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];
        let mut partida = PartidaMus::new_partida_lance(Lance::Punto, manos, [0, 0]).unwrap();
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0
        let _ = partida.actuar(Accion::Paso); // Pareja 1
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(), &[2, 0]);

        let manos = [
            Mano::try_from("1134").unwrap(),
            Mano::try_from("571S").unwrap(),
            Mano::try_from("3544").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];
        let mut partida = PartidaMus::new_partida_lance(Lance::Punto, manos, [0, 0]).unwrap();
        let _ = partida.actuar(Accion::Paso); // Pareja 0
        let _ = partida.actuar(Accion::Paso); // Pareja 0
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 1
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 1
        let _ = partida.actuar(Accion::Paso);
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(), &[0, 2]);
    }

    #[test]
    fn test_tanteo_limite() {
        let manos = [
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];

        // Grande
        let mut partida = PartidaMus::new(manos, [29, 32]);
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 1
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Paso); // Pareja 0
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(), &[29, 34]); // Pareja 1 + 2
                                                 // Chica
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 1
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero); // Pareja 0. 33, 34. Ganará la pareja 0 4 tantos al final.
        let _ = partida.actuar(Accion::Quiero);

        // Pares
        let _ = partida.actuar(Accion::Envido(2)); // Jugador 1
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Quiero); // Jugador 1
                                                // 40, 34. Ganará la pareja 0 4 tantos al final más 1 de par y 2 de medias. Total 7.

        // Juego
        let _ = partida.actuar(Accion::Envido(2)); // Jugador 1
        let _ = partida.actuar(Accion::Envido(2)); // Jugador 0
        let _ = partida.actuar(Accion::Quiero); // Jugador 1
                                                // 40, 40. anará la pareja 1 4 tantos al final, más 2 de juego. Total 6.
        assert_eq!(partida.tantos(), &[40, 0]);

        let manos = [
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];

        let mut partida = PartidaMus::new(manos, [29, 38]);
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 0
        let _ = partida.actuar(Accion::Envido(2)); // Pareja 1
        let _ = partida.actuar(Accion::Envido(2));
        let _ = partida.actuar(Accion::Paso); // Pareja 0
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.turno(), None);
        assert_eq!(partida.tantos(), &[0, 40]); // La pareja 1 gana 2 tantos y se va.
    }

    #[test]
    fn test_ordago() {
        let manos = [
            Mano::try_from("1234").unwrap(),
            Mano::try_from("57SS").unwrap(),
            Mano::try_from("3334").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];
        let mut partida = PartidaMus::new(manos, [0, 0]);
        let _ = partida.actuar(Accion::Ordago); // Pareja 0
        let _ = partida.actuar(Accion::Ordago); // Pareja 0
        let _ = partida.actuar(Accion::Paso); // Pareja 1
        let _ = partida.actuar(Accion::Paso);
        assert_eq!(partida.tantos(), &[1, 0]);
        let _ = partida.actuar(Accion::Ordago); // Pareja 0
        let _ = partida.actuar(Accion::Ordago); // Pareja 0
        let _ = partida.actuar(Accion::Quiero); // Pareja 1
        let _ = partida.actuar(Accion::Quiero);
        assert_eq!(partida.tantos(), &[40, 0]);
        assert_eq!(partida.turno(), None);
    }

    #[test]
    fn test_partida_lance() {
        let manos = [
            Mano::try_from("CC76").unwrap(),
            Mano::try_from("CCC1").unwrap(),
            Mano::try_from("1111").unwrap(),
            Mano::try_from("257C").unwrap(),
        ];
        let mut partida_lance = PartidaMus::new_partida_lance(Lance::Juego, manos, [0, 0]);
        assert!(partida_lance.is_some());
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        assert_eq!(partida_lance.as_ref().unwrap().lance_actual(), None);
        assert_eq!(partida_lance.as_ref().unwrap().tantos(), &[0, 3]);
        let manos = [
            Mano::try_from("257C").unwrap(),
            Mano::try_from("CC76").unwrap(),
            Mano::try_from("CCC1").unwrap(),
            Mano::try_from("1111").unwrap(),
        ];
        let mut partida_lance = PartidaMus::new_partida_lance(Lance::Juego, manos, [0, 0]);
        assert_eq!(
            partida_lance.as_ref().unwrap().turno(),
            Some(Turno::Jugador(1))
        );
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        let _ = partida_lance.as_mut().unwrap().actuar(Accion::Paso);
        assert_eq!(partida_lance.as_ref().unwrap().tantos(), &[3, 0]);
    }
}
