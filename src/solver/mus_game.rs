use arrayvec::ArrayString;
use itertools::Itertools;

use crate::{
    mus::{Accion, Apuesta, Baraja, Lance, Mano, PartidaMus, Turno},
    solver::ManosNormalizadas,
    Game, NodeType,
};

#[derive(Debug, Clone)]
pub struct MusGame {
    tantos: [u8; 2],
    partida: Option<PartidaMus>,
    history_str: ArrayString<64>,
    info_set_prefix: [ArrayString<16>; 4],
    last_action: Option<Accion>,
    manos_pares: ArrayString<4>,
    manos_juego: ArrayString<4>,
    abstract_game: bool,
}

impl MusGame {
    pub fn new(tantos: [u8; 2], abstract_game: bool) -> Self {
        Self {
            partida: None,
            tantos,
            history_str: ArrayString::new(),
            info_set_prefix: [ArrayString::new(); 4],
            last_action: None,
            manos_pares: ArrayString::new(),
            manos_juego: ArrayString::new(),
            abstract_game,
        }
    }

    fn from_partida_mus(partida: PartidaMus, abstract_game: bool) -> Self {
        if abstract_game {
            todo!("From partida mus not supported.")
        }
        let tantos = *partida.tantos();
        Self {
            partida: Some(partida),
            tantos,
            history_str: ArrayString::new(),
            info_set_prefix: [ArrayString::new(); 4],
            last_action: None,
            manos_pares: ArrayString::new(),
            manos_juego: ArrayString::new(),
            abstract_game,
        }
    }

    fn jugadas_manos(manos: &[Mano; 4]) -> (ArrayString<4>, ArrayString<4>) {
        let manos_pares = manos
            .iter()
            .map(|m| if m.pares().is_some() { '1' } else { '0' })
            .join("");
        let manos_juego = manos
            .iter()
            .map(|m| if m.pares().is_some() { '1' } else { '0' })
            .join("");

        (
            ArrayString::from(&manos_pares).unwrap(),
            ArrayString::from(&manos_juego).unwrap(),
        )
    }

    fn info_set_prefix(
        manos: &[Mano; 4],
        tantos: &[u8; 2],
        abstracto: Option<Lance>,
    ) -> [ArrayString<16>; 4] {
        let info_set_prefix: [ArrayString<16>; 4] = core::array::from_fn(|i| {
            if let Some(lance) = abstracto {
                let mano_abstracta = ManosNormalizadas::mano_to_abstract_string(&manos[i], &lance);
                ArrayString::from(&format!("{}:{},{},", tantos[0], tantos[1], mano_abstracta))
                    .unwrap()
            } else {
                ArrayString::from(&format!("{}:{},{},", tantos[0], tantos[1], manos[i])).unwrap()
            }
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

        payoff[player % 2] as f64
    }

    fn info_set_str(&self, player: usize) -> String {
        let mut output = String::with_capacity(15 + self.history_str.len());
        output.push_str(&self.info_set_prefix[player]);
        output.push_str(&self.history_str());
        output
    }

    fn new_random(&mut self) {
        let mut baraja = Baraja::baraja_mus();
        baraja.barajar();
        let manos = baraja.repartir_manos();
        self.info_set_prefix = MusGame::info_set_prefix(
            &manos,
            &self.tantos,
            if self.abstract_game {
                Some(Lance::Grande)
            } else {
                None
            },
        );
        (self.manos_pares, self.manos_juego) = MusGame::jugadas_manos(&manos);
        let partida = PartidaMus::new(manos, self.tantos);
        self.partida = Some(partida);
    }

    fn new_iter<F>(&mut self, _f: F)
    where
        F: FnMut(&mut Self, f64),
    {
        todo!()
    }

    fn actions(&self) -> Vec<Accion> {
        let partida = self.partida.as_ref().unwrap();
        let turno = partida.turno().unwrap();
        let ultimo_envite: Apuesta = partida.ultima_apuesta();
        let mut acciones = match ultimo_envite {
            Apuesta::Tantos(0) => vec![
                Accion::Paso,
                Accion::Envido(2),
                //Accion::Envido(5),
                //Accion::Envido(10),
                Accion::Ordago,
            ],
            Apuesta::Tantos(2) => vec![
                Accion::Paso,
                Accion::Quiero,
                Accion::Envido(2),
                //Accion::Envido(5),
                //Accion::Envido(10),
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
            .expect("At least one PartidaMus must be available.")
            .turno()
            .expect("One active player must be available.");
        match turno {
            Turno::Pareja(0 | 1) => {}
            _ => self.history_str.push_str(&a.to_string()),
        }

        let lance_previo = self.partida.as_mut().unwrap().lance_actual();
        let _ = self.partida.as_mut().unwrap().actuar(a);
        let lance_siguiente = self.partida.as_mut().unwrap().lance_actual();
        if lance_previo != lance_siguiente {
            if let Some(lance) = lance_siguiente {
                self.info_set_prefix = MusGame::info_set_prefix(
                    self.partida.as_ref().unwrap().manos(),
                    &self.tantos,
                    if self.abstract_game {
                        Some(lance)
                    } else {
                        None
                    },
                );
            }
            match lance_siguiente {
                Some(Lance::Pares) => self.history_str.push_str(self.manos_pares.as_str()),
                Some(Lance::Juego) => {
                    if lance_previo != Some(Lance::Pares) {
                        self.history_str.push_str(self.manos_pares.as_str());
                    }
                    self.history_str.push_str(self.manos_juego.as_str());
                }
                _ => {}
            }
        }
    }

    fn history_str(&self) -> String {
        let turno = self
            .partida
            .as_ref()
            .expect("At least one EstadoLance must be available.")
            .turno();
        match turno {
            Some(Turno::Pareja(2 | 3)) => {
                format!("{}{}*", self.history_str, self.last_action.unwrap())
            }
            _ => self.history_str.to_string(),
        }
    }
}
