use std::fmt::Display;
use std::fmt::Write;

use arrayvec::ArrayVec;
use serde::Deserialize;
use serde::Serialize;

use crate::mus::Carta;
use crate::mus::CuatroJugadores;
use crate::mus::DosJugadores;
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

    Mus,
    NoMus,
    Descartar([bool; 4]),
}

impl Display for Accion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Accion::Paso => f.write_char('p'),
            Accion::Envido(n) => f.write_fmt(format_args!("e{}", n)),
            Accion::Quiero => f.write_char('q'),
            Accion::Ordago => f.write_char('o'),
            Accion::Mus => f.write_char('m'),
            Accion::NoMus => f.write_char('n'),
            Accion::Descartar(d) => f.write_fmt(format_args!(
                "d{}",
                d.iter().map(|b| if *b { 1 } else { 0 }).sum::<i32>()
            )),
        }
    }
}

#[derive(Debug, Clone)]
struct ResultadoLance {
    ganador: u8,
    tantos: u8,
}

pub trait ModalidadMus: Sized {
    type N: AsRef<[Mano]> + Clone + std::fmt::Debug;

    fn tantos_ganador(lance: &Lance, manos: &Self::N, ganador: u8) -> u8;
    fn nuevo_estado_lance(lance: &Lance, manos: &Self::N, apuesta_maxima: u8) -> EstadoLance<Self>;
    fn actuar_envite(
        estado: &mut EstadoLance<Self>,
        accion: Accion,
    ) -> Result<Option<Turno>, MusError>;
    fn repartir_manos(baraja: &mut Baraja) -> Self::N;
}

impl ModalidadMus for DosJugadores {
    type N = [Mano; 2];

    fn tantos_ganador(lance: &Lance, manos: &Self::N, ganador: u8) -> u8 {
        lance.tantos_mano(&manos.as_ref()[ganador as usize]) + lance.bonus()
    }

    fn nuevo_estado_lance(lance: &Lance, manos: &Self::N, apuesta_maxima: u8) -> EstadoLance<Self> {
        EstadoLance::<DosJugadores>::new(lance, manos, apuesta_maxima)
    }

    fn actuar_envite(
        estado: &mut EstadoLance<Self>,
        accion: Accion,
    ) -> Result<Option<Turno>, MusError> {
        estado.actuar(accion)
    }

    fn repartir_manos(baraja: &mut Baraja) -> Self::N {
        baraja.repartir_manos()
    }
}

impl ModalidadMus for CuatroJugadores {
    type N = [Mano; 4];

    fn tantos_ganador(lance: &Lance, manos: &Self::N, ganador: u8) -> u8 {
        lance.tantos_mano(&manos.as_ref()[ganador as usize])
            + lance.tantos_mano(&manos.as_ref()[ganador as usize + 2])
            + lance.bonus()
    }

    fn nuevo_estado_lance(lance: &Lance, manos: &Self::N, apuesta_maxima: u8) -> EstadoLance<Self> {
        EstadoLance::<CuatroJugadores>::new(lance, manos, apuesta_maxima)
    }

    fn actuar_envite(
        estado: &mut EstadoLance<Self>,
        accion: Accion,
    ) -> Result<Option<Turno>, MusError> {
        estado.actuar(accion)
    }

    fn repartir_manos(baraja: &mut Baraja) -> Self::N {
        baraja.repartir_manos()
    }
}

#[derive(Debug, Clone)]
pub struct PartidaMus<T: ModalidadMus> {
    fase: Fase<T>,
}

impl<T: ModalidadMus> PartidaMus<T> {
    pub fn turno(&self) -> Option<Turno> {
        match &self.fase {
            Fase::Mus(fase_mus) => fase_mus.turno(),
            Fase::Envites(fase_envites) => fase_envites.turno(),
        }
    }

    pub fn fase(&self) -> Option<FasePartida> {
        match &self.fase {
            Fase::Mus(fase_mus) => match fase_mus.sub_fase {
                SubfaseMus::Mus => Some(FasePartida::Mus),
                SubfaseMus::Descartes => Some(FasePartida::Descartes),
                SubfaseMus::DescartePendiente {
                    jugador: _jugador,
                    descarte: _descarte,
                } => Some(FasePartida::DescartePendiente),
            },
            Fase::Envites(fase_envites) => fase_envites.lance_actual().map(FasePartida::Envites),
        }
    }

    pub fn tantos(&self) -> [u8; 2] {
        match &self.fase {
            Fase::Mus(fase_mus) => *fase_mus.tantos(),
            Fase::Envites(fase_envites) => *fase_envites.tantos(),
        }
    }

    pub fn fase_envites(&self) -> Option<&FaseEnvites<T>> {
        match &self.fase {
            Fase::Mus(_) => None,
            Fase::Envites(fase_envites) => Some(fase_envites),
        }
    }

    pub fn manos(&self) -> &T::N {
        match &self.fase {
            Fase::Mus(fase_mus) => &fase_mus.manos,
            Fase::Envites(fase_envites) => fase_envites.manos(),
        }
    }
}

impl PartidaMus<DosJugadores> {
    pub fn new(manos: [Mano; 2], tantos: [u8; 2]) -> Self {
        Self {
            fase: Fase::Mus(FaseMus::<DosJugadores>::new(manos, tantos)),
        }
    }

    pub fn actuar(&mut self, accion: Accion) -> Result<Option<Turno>, MusError> {
        match &mut self.fase {
            Fase::Mus(fase_mus) => {
                let turno = fase_mus.actuar(accion)?;
                if turno.is_some() {
                    return Ok(turno);
                }
                let manos = std::mem::take(&mut fase_mus.manos);
                let fase_envites = FaseEnvites::<DosJugadores>::new(manos, fase_mus.tantos);
                let turno = fase_envites.turno();
                self.fase = Fase::Envites(fase_envites);
                Ok(turno)
            }
            Fase::Envites(fase_envites) => fase_envites.actuar(accion),
        }
    }

    pub fn descartar_con_nuevas(&mut self, nuevas: &[Carta]) -> Result<Option<Turno>, MusError> {
        match &mut self.fase {
            Fase::Mus(fase_mus) => fase_mus.descartar_con_nuevas(nuevas),
            Fase::Envites(_) => Err(MusError::AccionNoValida),
        }
    }

    pub fn descartadas(&self) -> Result<ArrayVec<Carta, 4>, MusError> {
        match &self.fase {
            Fase::Mus(fase_mus) => fase_mus.descartadas(),
            Fase::Envites(_) => Err(MusError::AccionNoValida),
        }
    }
}

impl PartidaMus<CuatroJugadores> {
    pub fn new(manos: [Mano; 4], tantos: [u8; 2]) -> Self {
        Self {
            fase: Fase::Mus(FaseMus::<CuatroJugadores>::new(manos, tantos)),
        }
    }

    pub fn actuar(&mut self, accion: Accion) -> Result<Option<Turno>, MusError> {
        match &mut self.fase {
            Fase::Mus(fase_mus) => {
                let turno = fase_mus.actuar(accion)?;
                if turno.is_some() {
                    return Ok(turno);
                }
                let manos = std::mem::take(&mut fase_mus.manos);
                let fase_envites = FaseEnvites::<CuatroJugadores>::new(manos, fase_mus.tantos);
                let turno = fase_envites.turno();
                self.fase = Fase::Envites(fase_envites);
                Ok(turno)
            }
            Fase::Envites(fase_envites) => fase_envites.actuar(accion),
        }
    }

    pub fn descartar_con_nuevas(&mut self, nuevas: &[Carta]) -> Result<Option<Turno>, MusError> {
        match &mut self.fase {
            Fase::Mus(fase_mus) => fase_mus.descartar_con_nuevas(nuevas),
            Fase::Envites(_) => Err(MusError::AccionNoValida),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FasePartida {
    Mus,
    Descartes,
    DescartePendiente,
    Envites(Lance),
}

#[derive(Debug, Clone)]
enum Fase<T: ModalidadMus> {
    Mus(FaseMus<T>),
    Envites(FaseEnvites<T>),
}

#[derive(Debug, Clone)]
pub struct FaseMus<T: ModalidadMus> {
    manos: T::N,
    turno: Option<Turno>,
    sub_fase: SubfaseMus,
    tantos: [u8; 2],
}

#[derive(Debug, Clone)]
enum SubfaseMus {
    Mus,
    Descartes,
    DescartePendiente { jugador: u8, descarte: [bool; 4] },
}

impl<T: ModalidadMus> FaseMus<T> {
    pub fn turno(&self) -> Option<Turno> {
        self.turno
    }

    fn actuar_descartes(&mut self, accion: Accion) -> Result<Option<Turno>, MusError> {
        let Some(Turno::Jugador(t)) = self.turno else {
            panic!("Turno inválido en la fase de mus")
        };
        match accion {
            Accion::Descartar(descartes) => {
                if descartes == [false, false, false, false] {
                    return Err(MusError::AccionNoValida);
                }
                self.sub_fase = SubfaseMus::DescartePendiente {
                    jugador: t,
                    descarte: descartes,
                };
                Ok(self.turno)
            }
            _ => Err(MusError::AccionNoValida),
        }
    }

    pub fn tantos(&self) -> &[u8; 2] {
        &self.tantos
    }
}

impl FaseMus<DosJugadores> {
    pub fn new(manos: [Mano; 2], tantos: [u8; 2]) -> Self {
        Self {
            manos,
            turno: Some(Turno::Jugador(0)),
            sub_fase: SubfaseMus::Mus,
            tantos,
        }
    }

    pub fn actuar(&mut self, accion: Accion) -> Result<Option<Turno>, MusError> {
        match self.sub_fase {
            SubfaseMus::Mus => self.actuar_mus(accion),
            SubfaseMus::Descartes => self.actuar_descartes(accion),
            _ => Err(MusError::AccionNoValida),
        }
    }

    fn actuar_mus(&mut self, accion: Accion) -> Result<Option<Turno>, MusError> {
        match accion {
            Accion::Mus => {
                self.turno = match self.turno.ok_or(MusError::AccionNoValida)? {
                    Turno::Jugador(0) => Some(Turno::Jugador(1)),
                    Turno::Jugador(1) => {
                        self.sub_fase = SubfaseMus::Descartes;
                        Some(Turno::Jugador(0))
                    }
                    _ => panic!("Turno inválido en la fase de mus"),
                };
                return Ok(self.turno);
            }
            Accion::NoMus => return Ok(None),
            _ => return Err(MusError::AccionNoValida),
        }
    }

    fn descartadas(&self) -> Result<ArrayVec<Carta, 4>, MusError> {
        let SubfaseMus::DescartePendiente { jugador, descarte } = self.sub_fase else {
            return Err(MusError::AccionNoValida);
        };

        let mano = &self.manos[jugador as usize];
        Ok(mano
            .iter()
            .enumerate()
            .filter_map(|(idx, carta)| descarte[idx].then_some(*carta))
            .collect())
    }

    fn descartar_con_nuevas(&mut self, nuevas: &[Carta]) -> Result<Option<Turno>, MusError> {
        let SubfaseMus::DescartePendiente { jugador, descarte } = self.sub_fase else {
            return Err(MusError::AccionNoValida);
        };
        self.manos[jugador as usize].reemplazar(descarte, nuevas.iter().copied());
        self.turno = match self.turno.ok_or(MusError::AccionNoValida)? {
            Turno::Jugador(0) => {
                self.sub_fase = SubfaseMus::Descartes;
                Some(Turno::Jugador(1))
            }
            Turno::Jugador(1) => {
                self.sub_fase = SubfaseMus::Mus;
                Some(Turno::Jugador(0))
            }
            _ => panic!("Turno inválido en la fase de mus"),
        };
        Ok(self.turno)
    }
}

impl FaseMus<CuatroJugadores> {
    fn new(manos: [Mano; 4], tantos: [u8; 2]) -> Self {
        Self {
            manos,
            turno: Some(Turno::Pareja(0)),
            sub_fase: SubfaseMus::Mus,
            tantos,
        }
    }

    fn actuar(&mut self, accion: Accion) -> Result<Option<Turno>, MusError> {
        match self.sub_fase {
            SubfaseMus::Mus => self.actuar_mus(accion),
            SubfaseMus::Descartes => self.actuar_descartes(accion),
            _ => Err(MusError::AccionNoValida),
        }
    }

    fn actuar_mus(&mut self, accion: Accion) -> Result<Option<Turno>, MusError> {
        match accion {
            Accion::Mus => {
                self.turno = match self.turno.ok_or(MusError::AccionNoValida)? {
                    Turno::Pareja(0) => Some(Turno::Pareja(2)),
                    Turno::Pareja(2) => Some(Turno::Pareja(1)),
                    Turno::Pareja(1) => Some(Turno::Pareja(3)),
                    Turno::Pareja(3) => {
                        self.sub_fase = SubfaseMus::Descartes;
                        Some(Turno::Jugador(0))
                    }
                    _ => panic!("Turno inválido en la fase de mus"),
                };
                return Ok(self.turno);
            }
            Accion::NoMus => return Ok(None),
            _ => return Err(MusError::AccionNoValida),
        }
    }

    // fn descartar(&mut self) -> Result<Option<Turno>, MusError> {
    //     let SubfaseMus::DescartePendiente { jugador, descarte } = self.sub_fase else {
    //         return Err(MusError::AccionNoValida);
    //     };
    //     self.baraja
    //         .as_mut()
    //         .expect("Baraja must exist if descarte_pendiente() is called")
    //         .descartar(&mut self.manos[jugador as usize], descarte);
    //     self.turno = match self.turno.ok_or(MusError::AccionNoValida)? {
    //         Turno::Jugador(3) => {
    //             self.sub_fase = SubfaseMus::Mus;
    //             Some(Turno::Pareja(0))
    //         }
    //         Turno::Jugador(t) => {
    //             self.sub_fase = SubfaseMus::Descartes;
    //             Some(Turno::Jugador(t + 1))
    //         }
    //         _ => panic!("Turno inválido en la fase de mus"),
    //     };
    //     Ok(self.turno)
    // }

    fn descartar_con_nuevas(&mut self, nuevas: &[Carta]) -> Result<Option<Turno>, MusError> {
        let SubfaseMus::DescartePendiente { jugador, descarte } = self.sub_fase else {
            return Err(MusError::AccionNoValida);
        };
        self.manos[jugador as usize].reemplazar(descarte, nuevas.iter().copied());
        self.turno = match self.turno.ok_or(MusError::AccionNoValida)? {
            Turno::Jugador(3) => {
                self.sub_fase = SubfaseMus::Mus;
                Some(Turno::Pareja(0))
            }
            Turno::Jugador(t) => {
                self.sub_fase = SubfaseMus::Descartes;
                Some(Turno::Jugador(t + 1))
            }
            _ => panic!("Turno inválido en la fase de mus"),
        };
        Ok(self.turno)
    }
}

#[derive(Debug, Clone)]
pub struct FaseEnvites<T: ModalidadMus> {
    manos: T::N,
    lances: ArrayVec<(Lance, Option<ResultadoLance>), 4>,
    tantos: [u8; 2],
    idx_lance: usize,
    estado_lance: Option<EstadoLance<T>>,
}

impl<T: ModalidadMus> FaseEnvites<T> {
    pub const MAX_TANTOS: u8 = 40;

    /// Crea una partida de mus con las manos recibidas como parámetro. Las manos deben estar en un
    /// array y se asume que la primera posición se corresponde con la mano del jugador mano y la
    /// última con la del jugador postre.
    ///
    /// Recibe también los tantos con los que comienzan la partida
    /// cada una de las parejas.
    pub fn new(manos: T::N, tantos: [u8; 2]) -> Self {
        let mut lances = ArrayVec::new();
        lances.push((Lance::Grande, None));
        lances.push((Lance::Chica, None));
        if Lance::Pares.hay_lance(manos.as_ref()) {
            lances.push((Lance::Pares, None));
        }
        if Lance::Juego.hay_lance(manos.as_ref()) {
            lances.push((Lance::Juego, None));
        } else {
            lances.push((Lance::Punto, None));
        }
        let mut p = Self {
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
    pub fn new_partida_lance(lance: Lance, manos: T::N, tantos: [u8; 2]) -> Option<Self> {
        let mut lances = ArrayVec::<(Lance, Option<_>), 4>::new();
        lances.push((lance, None));
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

    fn crear_estado_lance(&self, l: Lance) -> EstadoLance<T> {
        let tantos_restantes = [
            Self::MAX_TANTOS - self.tantos[0],
            Self::MAX_TANTOS - self.tantos[1],
        ];
        let mut e = T::nuevo_estado_lance(
            &l,
            &self.manos,
            tantos_restantes[0].max(tantos_restantes[1]),
        );
        if !l.se_juega(self.manos.as_ref()) {
            e.resolver_lance();
        }
        e
    }
    /// Realiza la acción recibida como parámetro. Devuelve el turno de la siguiente pareja o Ok(None)
    /// si la partida ha terminado. Esta función devuelve error si se llama tras haber acabado la
    /// partida.
    pub fn actuar(&mut self, accion: Accion) -> Result<Option<Turno>, MusError> {
        let estado_lance = self.estado_lance.as_mut().ok_or(MusError::AccionNoValida)?;
        let turno = T::actuar_envite(estado_lance, accion)?;
        if turno.is_some() {
            return Ok(turno);
        }
        let lance = self.lances[self.idx_lance].0;
        self.tanteo_envites_lance();
        self.tanteo_final_lance(&lance);
        while let Some((lance, estado_lance)) = self.siguiente_lance() {
            if estado_lance.turno().is_some() {
                return Ok(estado_lance.turno());
            } else {
                self.tanteo_final_lance(&lance);
            }
        }
        self.tanteo_final();
        Ok(None)
    }

    fn siguiente_lance(&mut self) -> Option<(Lance, &EstadoLance<T>)> {
        self.estado_lance.as_ref()?;
        if self.idx_lance < self.lances.len() - 1 {
            self.idx_lance += 1;
            let lance = self.lances[self.idx_lance].0;
            let estado_lance = self.crear_estado_lance(lance);
            self.estado_lance = Some(estado_lance);
            Some(lance).zip(self.estado_lance.as_ref())
        } else {
            self.estado_lance = None;
            None
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

    /// Devuelve la apuesta máxima del lance en curso. Si la partida ha terminado devuelve 0.
    pub fn apuesta_maxima(&self) -> u8 {
        self.estado_lance
            .as_ref()
            .map_or_else(|| 0, |estado_lance| estado_lance.apuesta_maxima())
    }

    /// Devuelve las manos de los jugadores.
    pub fn manos(&self) -> &T::N {
        &self.manos
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

            tantos += T::tantos_ganador(l, &self.manos, ganador);
            self.lances[self.idx_lance].1 = Some(ResultadoLance { ganador, tantos });
        }
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

        let mut partida = FaseEnvites::<CuatroJugadores>::new(manos, [0, 0]);
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

        let mut partida = FaseEnvites::<CuatroJugadores>::new(manos, [0, 0]);
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
        let mut partida =
            FaseEnvites::<CuatroJugadores>::new_partida_lance(Lance::Punto, manos, [0, 0]).unwrap();
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
        let mut partida =
            FaseEnvites::<CuatroJugadores>::new_partida_lance(Lance::Punto, manos, [0, 0]).unwrap();
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
        let mut partida = FaseEnvites::<CuatroJugadores>::new(manos, [29, 32]);
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

        let mut partida = FaseEnvites::<CuatroJugadores>::new(manos, [29, 38]);
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
        let mut partida = FaseEnvites::<CuatroJugadores>::new(manos, [0, 0]);
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
        let mut partida_lance =
            FaseEnvites::<CuatroJugadores>::new_partida_lance(Lance::Juego, manos, [0, 0]);
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
        let mut partida_lance =
            FaseEnvites::<CuatroJugadores>::new_partida_lance(Lance::Juego, manos, [0, 0]);
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

    #[test]
    fn test_marcador_sin_lances() {
        let manos = [
            Mano::try_from("RS64").unwrap(),
            Mano::try_from("RCC1").unwrap(),
        ];
        let mut game = FaseEnvites::<DosJugadores>::new(manos, [0, 0]);
        let _ = game.actuar(Accion::Paso);
        let _ = game.actuar(Accion::Envido(2));
        let _ = game.actuar(Accion::Paso);
        assert_eq!(game.tantos(), &[0, 1]);
        let _ = game.actuar(Accion::Paso);
        let _ = game.actuar(Accion::Envido(2));
        let turno = game.actuar(Accion::Paso);
        // Solo el jugador 2 tiene pares y juego, la partida termina.
        assert!(turno.is_ok());
        assert!(turno.unwrap().is_none());
        // 2 tantos de envites en grande y chica, 1 de pares, 3 de juego.
        assert_eq!(game.tantos(), &[0, 6]);

        let manos = [
            Mano::try_from("RS64").unwrap(),
            Mano::try_from("RCC1").unwrap(),
            Mano::try_from("RS64").unwrap(),
            Mano::try_from("RCC1").unwrap(),
        ];
        let mut game = FaseEnvites::<CuatroJugadores>::new(manos, [0, 0]);
        let _ = game.actuar(Accion::Paso);
        let _ = game.actuar(Accion::Paso);
        let _ = game.actuar(Accion::Envido(2));
        let _ = game.actuar(Accion::Envido(2));
        let _ = game.actuar(Accion::Paso);
        let _ = game.actuar(Accion::Paso);
        assert_eq!(game.tantos(), &[0, 1]);
        let _ = game.actuar(Accion::Paso);
        let _ = game.actuar(Accion::Paso);
        let _ = game.actuar(Accion::Envido(2));
        let _ = game.actuar(Accion::Envido(2));
        let _ = game.actuar(Accion::Paso);
        let turno = game.actuar(Accion::Paso);
        // Solo el jugador 2 tiene pares y juego, la partida termina.
        assert!(turno.is_ok());
        assert!(turno.unwrap().is_none());
        // 2 tantos de envites en grande y chica, 2 de pares, 6 de juego.
        assert_eq!(game.tantos(), &[0, 10]);
    }

    #[test]
    fn test_fase_mus() {
        let mut partida = PartidaMus::<CuatroJugadores>::new(
            [
                Mano::try_from("RRAA").unwrap(),
                Mano::try_from("RRAA").unwrap(),
                Mano::try_from("RRAA").unwrap(),
                Mano::try_from("RRAA").unwrap(),
            ],
            [0, 0],
        );

        assert_eq!(partida.fase(), Some(FasePartida::Mus));
        assert_eq!(partida.turno(), Some(Turno::Pareja(0)));

        let turno = partida.actuar(Accion::Paso);
        assert!(turno.is_err());
        let turno = partida.actuar(Accion::Descartar([false, true, true, true]));
        assert!(turno.is_err());
        let _ = partida.actuar(Accion::NoMus);
        assert_eq!(partida.fase(), Some(FasePartida::Envites(Lance::Grande)));

        let mut partida = PartidaMus::<CuatroJugadores>::new(
            [
                Mano::try_from("RRAA").unwrap(),
                Mano::try_from("RRAA").unwrap(),
                Mano::try_from("RRAA").unwrap(),
                Mano::try_from("RRAA").unwrap(),
            ],
            [0, 0],
        );
        let _ = partida.actuar(Accion::Mus);
        assert_eq!(partida.turno(), Some(Turno::Pareja(2)));
        let _ = partida.actuar(Accion::Mus);
        assert_eq!(partida.turno(), Some(Turno::Pareja(1)));
        let _ = partida.actuar(Accion::Mus);
        assert_eq!(partida.turno(), Some(Turno::Pareja(3)));
        let _ = partida.actuar(Accion::Mus);
        assert_eq!(partida.fase(), Some(FasePartida::Descartes));
        assert_eq!(partida.turno(), Some(Turno::Jugador(0)));

        let turno = partida.actuar(Accion::Descartar([false, false, false, false]));
        assert!(turno.is_err());
        let turno = partida.actuar(Accion::Descartar([true, false, false, false]));
        assert!(turno.is_ok());
        let _ = partida.descartar_con_nuevas(&[Carta::Rey]);
        assert_eq!(partida.turno(), Some(Turno::Jugador(1)));
        let _ = partida.actuar(Accion::Descartar([true, false, false, false]));
        let _ = partida.descartar_con_nuevas(&[Carta::Rey]);
        let _ = partida.actuar(Accion::Descartar([true, false, false, false]));
        let _ = partida.descartar_con_nuevas(&[Carta::Rey]);
        let _ = partida.actuar(Accion::Descartar([true, false, false, false]));
        let _ = partida.descartar_con_nuevas(&[Carta::Rey]);
        assert_eq!(partida.fase(), Some(FasePartida::Mus));
        assert_eq!(partida.turno(), Some(Turno::Pareja(0)));
    }
}
