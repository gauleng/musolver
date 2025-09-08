use arrayvec::{ArrayString, ArrayVec};
use itertools::Itertools;

use crate::{
    mus::{Accion, Apuesta, Baraja, Carta, Lance, Mano, PartidaMus, Turno},
    Game, NodeType,
};

use super::ManosNormalizadas;

#[derive(Debug, Clone)]
pub struct MusGame {
    tantos: [u8; 2],
    partida: Option<PartidaMus>,
    history_str: ArrayVec<ArrayString<4>, 14>,
    info_set_prefix: [ArrayString<64>; 4],
    last_action: Option<Accion>,
    abstract_game: bool,
}

impl MusGame {
    pub fn new(tantos: [u8; 2], abstract_game: bool) -> Self {
        Self {
            partida: None,
            tantos,
            history_str: ArrayVec::new(),
            info_set_prefix: [ArrayString::<64>::new(); 4],
            last_action: None,
            abstract_game,
        }
    }

    fn from_partida_mus(partida: PartidaMus, abstract_game: bool) -> Self {
        let tantos = *partida.tantos();
        Self {
            partida: Some(partida),
            tantos,
            history_str: ArrayVec::new(),
            info_set_prefix: [ArrayString::<64>::new(); 4],
            last_action: None,
            abstract_game,
        }
    }

    fn info_set_prefix(
        manos: &[Mano; 4],
        tantos: &[u8; 2],
        abstracto: bool,
    ) -> [ArrayString<64>; 4] {
        let info_set_prefix: [ArrayString<64>; 4] = core::array::from_fn(|i| {
            ArrayString::<64>::from(&format!("{}:{},{},", tantos[0], tantos[1], manos[i])).unwrap()
        });
        info_set_prefix
    }
}

impl Game for MusGame {
    type Action = Accion;
    const N_PLAYERS: usize = 4;

    fn utility(&mut self, player: usize) -> f64 {
        let tantos = self.partida.as_mut().unwrap().tantos();

        let payoff = [
            tantos[0] as i8 - tantos[1] as i8,
            tantos[1] as i8 - tantos[0] as i8,
        ];

        payoff[player] as f64
    }

    fn info_set_str(&self, player: usize) -> String {
        let mut output = String::with_capacity(15 + self.history_str.len() + 1);
        output.push_str(&self.tantos[0].to_string());
        output.push(':');
        output.push_str(&self.tantos[1].to_string());
        output.push(',');
        for i in self.history_str.iter() {
            output.push_str(&i.to_string());
        }
        output
    }

    fn new_random(&mut self) {
        let mut baraja = Baraja::baraja_mus();
        baraja.barajar();
        let manos = baraja.repartir_manos();
        self.info_set_prefix = MusGame::info_set_prefix(&manos, &self.tantos, self.abstract_game);
        let partida = PartidaMus::new(manos, self.tantos);
        self.partida = Some(partida);
    }

    fn new_iter<F>(&mut self, _f: F)
    where
        F: FnMut(&mut Self, f64),
    {
        todo!()
    }

    fn num_players(&self) -> usize {
        4
    }

    fn actions(&self) -> Vec<Accion> {
        let partida = self.partida.as_ref().unwrap();
        let turno = partida.turno().unwrap();
        let ultimo_envite: Apuesta = partida.ultima_apuesta();
        let mut acciones = match ultimo_envite {
            Apuesta::Tantos(0) => vec![
                Accion::Paso,
                Accion::Envido(2),
                Accion::Envido(5),
                Accion::Envido(10),
                Accion::Ordago,
            ],
            Apuesta::Tantos(2) => vec![
                Accion::Paso,
                Accion::Quiero,
                Accion::Envido(2),
                Accion::Envido(5),
                Accion::Envido(10),
                Accion::Ordago,
            ],
            Apuesta::Tantos(4..=5) => vec![
                Accion::Paso,
                Accion::Quiero,
                Accion::Envido(10),
                Accion::Ordago,
            ],
            Apuesta::Ordago => vec![Accion::Paso, Accion::Quiero],
            _ => vec![Accion::Paso, Accion::Quiero, Accion::Ordago],
        };
        if turno == Turno::Pareja(2) || turno == Turno::Pareja(3) {
            acciones.retain(|a| a >= self.last_action.as_ref().unwrap());
        }
        acciones
    }

    fn current_player(&self) -> NodeType {
        match &self.partida {
            None => NodeType::Chance,
            Some(estado_lance) => match estado_lance.turno() {
                None => NodeType::Terminal,
                Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                    NodeType::Player(player_id as usize)
                }
            },
        }
    }

    fn act(&mut self, a: Accion) {
        self.last_action = Some(a);
        let turno = self
            .partida
            .as_ref()
            .expect("At least one EstadoLance must be available.")
            .turno()
            .unwrap();
        match turno {
            Turno::Pareja(2) | Turno::Pareja(3) => {
                self.history_str.pop();
            }
            _ => {}
        };
        let action_str = match turno {
            Turno::Pareja(0) | Turno::Pareja(1) => a.to_string() + "*",
            _ => a.to_string(),
        };
        self.history_str
            .push(ArrayString::<4>::from(&action_str).unwrap());
        let _ = self.partida.as_mut().unwrap().actuar(a);
    }

    fn history_str(&self) -> String {
        todo!()
    }
}
