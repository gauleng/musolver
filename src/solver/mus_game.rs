use std::{fmt::Write, rc::Rc};

use arrayvec::ArrayString;
use itertools::Itertools;

use crate::{
    Game, NodeType,
    mus::{
        Accion, Apuesta, Baraja, CuatroJugadores, DistribucionDobleCartaIter, DosJugadores, Lance,
        Mano, PartidaMus, Turno,
    },
    solver::ManosNormalizadas,
};

#[derive(Debug, Clone)]
pub struct MusGame {
    tantos: [u8; 2],
    partida: Option<PartidaMus<CuatroJugadores>>,
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

    pub fn new_with_hands(manos: &[Mano; 4], tantos: [u8; 2], abstract_game: bool) -> Self {
        let manos = [
            manos[0].clone(),
            manos[1].clone(),
            manos[2].clone(),
            manos[3].clone(),
        ];
        let info_set_prefix = MusGame::info_set_prefix(
            &manos,
            &tantos,
            if abstract_game {
                Some(Lance::Grande)
            } else {
                None
            },
        );
        let (manos_pares, manos_juego) = MusGame::jugadas_manos(&manos);
        let partida = Some(PartidaMus::<CuatroJugadores>::new(manos, tantos));
        let history_str = ArrayString::<64>::from("M").unwrap();
        Self {
            partida,
            tantos,
            history_str,
            info_set_prefix,
            manos_pares,
            manos_juego,
            abstract_game,
            last_action: None,
        }
    }
    /*fn from_partida_mus(partida: PartidaMus, abstract_game: bool) -> Self {
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
    }*/

    fn jugadas_manos(manos: &[Mano; 4]) -> (ArrayString<4>, ArrayString<4>) {
        let manos_pares = manos
            .iter()
            .map(|m| if m.pares().is_some() { '1' } else { '0' })
            .join("");
        let manos_juego = manos
            .iter()
            .map(|m| if m.juego().is_some() { '1' } else { '0' })
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
        let baraja = Baraja::baraja_mus();
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
        let partida = PartidaMus::<CuatroJugadores>::new(manos, self.tantos);
        self.partida = Some(partida);
        self.history_str.push('M');
    }

    fn reset(&mut self) {
        self.partida = None;
        self.history_str.clear();
        self.info_set_prefix = [ArrayString::new(); 4];
        self.last_action = None;
        self.manos_pares.clear();
        self.manos_juego.clear();
    }

    fn new_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        DistribucionDobleCartaIter::new(&Baraja::FREC_BARAJA_MUS).flat_map(
            move |(mano1, mano2, prob)| {
                DistribucionDobleCartaIter::new(&Baraja::FREC_BARAJA_MUS).map(
                    move |(mano3, mano4, prob2)| {
                        let manos = [
                            Mano::new(mano1.to_owned()),
                            Mano::new(mano2.to_owned()),
                            Mano::new(mano3),
                            Mano::new(mano4),
                        ];

                        (
                            Self::new_with_hands(&manos, self.tantos, self.abstract_game),
                            prob * prob2,
                        )
                    },
                )
            },
        )
    }

    fn actions(&self) -> Vec<Accion> {
        let partida = self.partida.as_ref().unwrap();
        let turno = partida.turno().unwrap();
        let ultimo_envite: Apuesta = partida.ultima_apuesta();
        let tantos = partida.tantos();
        let max_tantos = tantos[0].max(tantos[1]);
        let mut acciones = if max_tantos >= 38 {
            match ultimo_envite {
                Apuesta::Tantos(0) => vec![Accion::Paso, Accion::Ordago],
                Apuesta::Ordago => vec![Accion::Paso, Accion::Quiero],
                _ => vec![Accion::Paso, Accion::Quiero, Accion::Ordago],
            }
        } else {
            match ultimo_envite {
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
            }
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
                Some(Lance::Punto) | Some(Lance::Juego) => {
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
        if let Some(partida) = self.partida.as_ref() {
            match partida.turno() {
                Some(Turno::Pareja(2 | 3)) => {
                    format!("{}{}*", self.history_str, self.last_action.unwrap())
                }
                _ => self.history_str.to_string(),
            }
        } else {
            "".into()
        }
    }
}

#[derive(Debug, Clone)]
pub struct MusGameTwoHands {
    tantos: [u8; 2],
    partida: Option<PartidaMus<CuatroJugadores>>,
    history_str: ArrayString<64>,
    info_set_prefix: [ArrayString<16>; 2],
    manos_pares: ArrayString<4>,
    manos_juego: ArrayString<4>,
    abstract_game: bool,
}

impl MusGameTwoHands {
    pub fn new(tantos: [u8; 2], abstract_game: bool) -> Self {
        Self {
            partida: None,
            tantos,
            history_str: ArrayString::new(),
            info_set_prefix: [ArrayString::new(); 2],
            manos_pares: ArrayString::new(),
            manos_juego: ArrayString::new(),
            abstract_game,
        }
    }

    pub fn new_with_hands(manos: &[Mano; 4], tantos: [u8; 2], abstract_game: bool) -> Self {
        let manos = [
            manos[0].clone(),
            manos[1].clone(),
            manos[2].clone(),
            manos[3].clone(),
        ];
        let info_set_prefix = MusGameTwoHands::info_set_prefix(
            &manos,
            &tantos,
            if abstract_game {
                Some(Lance::Grande)
            } else {
                None
            },
        );
        let (manos_pares, manos_juego) = MusGame::jugadas_manos(&manos);
        let partida = Some(PartidaMus::<CuatroJugadores>::new(manos, tantos));
        let history_str = ArrayString::<64>::from("M").unwrap();
        Self {
            partida,
            tantos,
            history_str,
            info_set_prefix,
            manos_pares,
            manos_juego,
            abstract_game,
        }
    }

    fn info_set_prefix(
        manos: &[Mano; 4],
        tantos: &[u8; 2],
        abstracto: Option<Lance>,
    ) -> [ArrayString<16>; 2] {
        let info_set_prefix: [ArrayString<16>; 2] = core::array::from_fn(|i| {
            if let Some(lance) = abstracto {
                let mano_abstracta = ManosNormalizadas::mano_to_abstract_string(&manos[i], &lance);
                let mano_abstracta2 =
                    ManosNormalizadas::mano_to_abstract_string(&manos[i + 2], &lance);
                ArrayString::from(&format!(
                    "{}:{},{},{},",
                    tantos[0], tantos[1], mano_abstracta, mano_abstracta2
                ))
                .unwrap()
            } else {
                ArrayString::from(&format!(
                    "{}:{},{},{},",
                    tantos[0],
                    tantos[1],
                    manos[i],
                    manos[i + 2]
                ))
                .unwrap()
            }
        });
        info_set_prefix
    }
}

impl Game for MusGameTwoHands {
    type Action = Accion;
    const N_PLAYERS: usize = 2;

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
        let baraja = Baraja::baraja_mus();
        let manos = baraja.repartir_manos();
        self.info_set_prefix = MusGameTwoHands::info_set_prefix(
            &manos,
            &self.tantos,
            if self.abstract_game {
                Some(Lance::Grande)
            } else {
                None
            },
        );
        (self.manos_pares, self.manos_juego) = MusGame::jugadas_manos(&manos);
        let partida = PartidaMus::<CuatroJugadores>::new(manos, self.tantos);
        self.partida = Some(partida);
        self.history_str.push('M');
    }

    fn reset(&mut self) {
        self.partida = None;
        self.history_str.clear();
        self.info_set_prefix = [ArrayString::new(); 2];
        self.manos_pares.clear();
        self.manos_juego.clear();
    }

    fn new_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        DistribucionDobleCartaIter::new(&Baraja::FREC_BARAJA_MUS).flat_map(
            move |(mano1, mano2, prob)| {
                DistribucionDobleCartaIter::new(&Baraja::FREC_BARAJA_MUS).map(
                    move |(mano3, mano4, prob2)| {
                        let manos = [
                            Mano::new(mano1.to_owned()),
                            Mano::new(mano2.to_owned()),
                            Mano::new(mano3),
                            Mano::new(mano4),
                        ];
                        (
                            Self::new_with_hands(&manos, self.tantos, self.abstract_game),
                            prob * prob2,
                        )
                    },
                )
            },
        )
    }

    fn actions(&self) -> Vec<Accion> {
        let partida = self.partida.as_ref().unwrap();
        let ultimo_envite: Apuesta = partida.ultima_apuesta();
        let tantos = partida.tantos();
        let max_tantos = tantos[0].max(tantos[1]);
        if max_tantos >= 38 {
            match ultimo_envite {
                Apuesta::Tantos(0) => vec![Accion::Paso, Accion::Ordago],
                Apuesta::Ordago => vec![Accion::Paso, Accion::Quiero],
                _ => vec![Accion::Paso, Accion::Quiero, Accion::Ordago],
            }
        } else {
            match ultimo_envite {
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
            }
        }
    }

    fn current_player(&self) -> NodeType {
        match &self.partida {
            None => NodeType::Chance,
            Some(estado_lance) => match estado_lance.turno() {
                None => NodeType::Terminal,
                Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                    NodeType::Player(player_id as usize % 2)
                }
            },
        }
    }

    fn act(&mut self, a: Accion) {
        self.history_str.push_str(&a.to_string());
        if let Some(partida) = self.partida.as_mut() {
            let lance_previo = partida.lance_actual();
            if let Turno::Pareja(_) = partida.turno().expect("some player must be playing") {
                let _ = partida.actuar(a);
            }
            let _ = partida.actuar(a);
            let lance_siguiente = partida.lance_actual();
            if lance_previo != lance_siguiente {
                if let Some(lance) = lance_siguiente {
                    self.info_set_prefix = MusGameTwoHands::info_set_prefix(
                        partida.manos(),
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
                    Some(Lance::Punto) | Some(Lance::Juego) => {
                        if lance_previo != Some(Lance::Pares) {
                            self.history_str.push_str(self.manos_pares.as_str());
                        }
                        self.history_str.push_str(self.manos_juego.as_str());
                    }
                    _ => {}
                }
            }
        }
    }

    fn history_str(&self) -> String {
        self.history_str.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct MusGameTwoPlayers {
    tantos: [u8; 2],
    partida: Option<PartidaMus<DosJugadores>>,
    history_str: ArrayString<64>,
    info_set_prefix: [ArrayString<16>; 2],
    manos_pares: ArrayString<2>,
    manos_juego: ArrayString<2>,
    abstract_game: bool,
    utility_table: Rc<[[f64; 40]; 40]>,
}

impl MusGameTwoPlayers {
    pub fn new(tantos: [u8; 2], abstract_game: bool, utility_table: Rc<[[f64; 40]; 40]>) -> Self {
        Self {
            partida: None,
            tantos,
            history_str: ArrayString::new(),
            info_set_prefix: [ArrayString::new(); 2],
            manos_pares: ArrayString::new(),
            manos_juego: ArrayString::new(),
            abstract_game,
            utility_table,
        }
    }

    pub fn new_with_hands(manos: &[Mano; 2], tantos: [u8; 2], abstract_game: bool) -> Self {
        let manos = [manos[0].clone(), manos[1].clone()];
        let info_set_prefix = MusGameTwoPlayers::info_set_prefix(
            &manos,
            &tantos,
            if abstract_game {
                Some(Lance::Grande)
            } else {
                None
            },
        );
        let (manos_pares, manos_juego) = MusGameTwoPlayers::jugadas_manos(&manos);
        let partida = Some(PartidaMus::<DosJugadores>::new(manos, tantos));
        let history_str = ArrayString::<64>::from("M").unwrap();
        Self {
            partida,
            tantos,
            history_str,
            info_set_prefix,
            manos_pares,
            manos_juego,
            abstract_game,
            utility_table: Rc::new([[0.; 40]; 40]),
        }
    }

    pub fn mus_game(&self) -> Option<&PartidaMus<DosJugadores>> {
        self.partida.as_ref()
    }

    fn jugadas_manos(manos: &[Mano; 2]) -> (ArrayString<2>, ArrayString<2>) {
        let hay_pares = ArrayString::from(match (manos[0].hay_pares(), manos[1].hay_pares()) {
            (true, true) => "11",
            (true, false) => "10",
            (false, true) => "01",
            (false, false) => "00",
        });
        let hay_juego = ArrayString::from(
            match (manos[0].juego().is_some(), manos[1].juego().is_some()) {
                (true, true) => "11",
                (true, false) => "10",
                (false, true) => "01",
                (false, false) => "00",
            },
        );

        (hay_pares.unwrap(), hay_juego.unwrap())
    }

    fn info_set_prefix(
        manos: &[Mano; 2],
        tantos: &[u8; 2],
        abstracto: Option<Lance>,
    ) -> [ArrayString<16>; 2] {
        let info_set_prefix: [ArrayString<16>; 2] = core::array::from_fn(|i| {
            if let Some(lance) = abstracto {
                let mano_abstracta = ManosNormalizadas::mano_to_abstract_string(&manos[i], &lance);
                let mut str = ArrayString::new();
                let _ = write!(&mut str, "{}:{},{},", tantos[0], tantos[1], mano_abstracta);

                str
            } else {
                let mut str = ArrayString::new();
                let _ = write!(&mut str, "{}:{},{},", tantos[0], tantos[1], manos[i]);

                str
            }
        });
        info_set_prefix
    }

    pub fn default_utility_table() -> [[f64; 40]; 40] {
        std::array::from_fn(|t1| std::array::from_fn(|t2| (t1 - t2) as f64))
    }
}

impl Game for MusGameTwoPlayers {
    type Action = Accion;
    const N_PLAYERS: usize = 2;

    fn utility(&mut self, player: usize) -> f64 {
        let tantos = self.partida.as_mut().unwrap().tantos();

        if tantos[0] == 40 || tantos[1] == 40 {
            let payoff = [
                tantos[0] as i8 - tantos[1] as i8,
                tantos[1] as i8 - tantos[0] as i8,
            ];

            payoff[player % 2] as f64
        } else {
            let expected_utility = self.utility_table[tantos[1] as usize][tantos[0] as usize];
            if player == 0 {
                -expected_utility
            } else {
                expected_utility
            }
        }
    }

    fn info_set_str(&self, player: usize) -> String {
        let mut output = String::with_capacity(15 + self.history_str.len());
        output.push_str(&self.info_set_prefix[player]);
        output.push_str(&self.history_str());
        output
    }

    fn new_random(&mut self) {
        let baraja = Baraja::baraja_mus();
        let manos = baraja.repartir_manos();
        let manos = [manos[0].clone(), manos[1].clone()];
        self.info_set_prefix = MusGameTwoPlayers::info_set_prefix(
            &manos,
            &self.tantos,
            if self.abstract_game {
                Some(Lance::Grande)
            } else {
                None
            },
        );
        (self.manos_pares, self.manos_juego) = MusGameTwoPlayers::jugadas_manos(&manos);
        let partida = PartidaMus::<DosJugadores>::new(manos, self.tantos);
        self.partida = Some(partida);
        self.history_str.push('M');
    }

    fn reset(&mut self) {
        self.partida = None;
        self.history_str.clear();
        self.info_set_prefix = [ArrayString::new(); 2];
        self.manos_pares.clear();
        self.manos_juego.clear();
    }

    fn new_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        DistribucionDobleCartaIter::new(&Baraja::FREC_BARAJA_MUS).map(
            |(mano1, mano2, probability)| {
                (
                    Self::new_with_hands(
                        &[Mano::new(mano1), Mano::new(mano2)],
                        self.tantos,
                        self.abstract_game,
                    ),
                    probability,
                )
            },
        )
    }

    fn actions(&self) -> Vec<Accion> {
        let partida = self.partida.as_ref().unwrap();
        let ultimo_envite: Apuesta = partida.ultima_apuesta();
        let apuesta_maxima = partida.apuesta_maxima();
        let mut actions = match ultimo_envite {
            Apuesta::Tantos(tantos) if tantos == apuesta_maxima => {
                return vec![Accion::Paso, Accion::Quiero, Accion::Ordago];
            }
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
            Apuesta::Ordago => return vec![Accion::Paso, Accion::Quiero],
            _ => return vec![Accion::Paso, Accion::Quiero, Accion::Ordago],
        };
        actions.retain(|action| {
            if let Apuesta::Tantos(tantos) = ultimo_envite
                && let Accion::Envido(v) = action
            {
                tantos + *v < apuesta_maxima
            } else {
                true
            }
        });
        actions
    }

    fn current_player(&self) -> NodeType {
        match &self.partida {
            None => NodeType::Chance,
            Some(estado_lance) => match estado_lance.turno() {
                None => NodeType::Terminal,
                Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                    NodeType::Player(player_id as usize % 2)
                }
            },
        }
    }

    fn act(&mut self, a: Accion) {
        self.history_str.push_str(&a.to_string());
        if let Some(partida) = self.partida.as_mut() {
            let lance_previo = partida.lance_actual();
            if let Turno::Pareja(_) = partida.turno().expect("some player must be playing") {
                let _ = partida.actuar(a);
            }
            let _ = partida.actuar(a);
            let lance_siguiente = partida.lance_actual();
            if lance_previo != lance_siguiente {
                if let Some(lance) = lance_siguiente {
                    self.info_set_prefix = MusGameTwoPlayers::info_set_prefix(
                        partida.manos(),
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
                    Some(Lance::Punto) | Some(Lance::Juego) => {
                        if lance_previo != Some(Lance::Pares) {
                            self.history_str.push_str(self.manos_pares.as_str());
                        }
                        self.history_str.push_str(self.manos_juego.as_str());
                    }
                    _ => {}
                }
            }
        }
    }

    fn history_str(&self) -> String {
        self.history_str.to_string()
    }
}
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn two_players_infoset() {
        let manos = [
            Mano::from_str("RRR5").unwrap(),
            Mano::from_str("RCC1").unwrap(),
        ];
        let mut game = MusGameTwoPlayers::new_with_hands(&manos, [38, 37], false);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,M");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(1), "38:37,RCC1,Mp");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mpp");
        game.act(Accion::Paso);
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mpppo");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mpppop11");
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(1), "38:37,RCC1,Mpppop11o");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mpppop11op11");

        let manos = [
            Mano::from_str("R775").unwrap(),
            Mano::from_str("S651").unwrap(),
        ];
        let mut game = MusGameTwoPlayers::new_with_hands(&manos, [38, 37], false);
        game.act(Accion::Paso);
        game.act(Accion::Paso);

        game.act(Accion::Paso);
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(0), "38:37,R775,Mpppo");
        game.act(Accion::Paso);

        assert_eq!(game.info_set_str(0), "38:37,R775,Mpppop1000");
    }

    #[test]
    fn two_players_actions() {
        let manos = [
            Mano::from_str("RRR5").unwrap(),
            Mano::from_str("RCC1").unwrap(),
        ];
        let mut game = MusGameTwoPlayers::new_with_hands(&manos, [35, 35], false);
        game.act(Accion::Envido(2));
        game.act(Accion::Envido(2));
        assert_eq!(
            game.actions(),
            vec![Accion::Paso, Accion::Quiero, Accion::Ordago,]
        );
        game.act(Accion::Quiero);
        game.act(Accion::Envido(10));
        assert_eq!(
            game.actions(),
            vec![Accion::Paso, Accion::Quiero, Accion::Ordago,]
        );
        let mut game = MusGameTwoPlayers::new_with_hands(&manos, [37, 37], false);
        assert_eq!(
            game.actions(),
            vec![Accion::Paso, Accion::Envido(2), Accion::Ordago,]
        );
        game.act(Accion::Envido(2));
        assert_eq!(
            game.actions(),
            vec![Accion::Paso, Accion::Quiero, Accion::Ordago,]
        );
        game.act(Accion::Paso);
        assert_eq!(
            game.actions(),
            vec![Accion::Paso, Accion::Envido(2), Accion::Ordago,]
        );
        game.act(Accion::Paso);
        game.act(Accion::Envido(2));
        game.act(Accion::Paso);
        assert_eq!(game.actions(), vec![Accion::Paso, Accion::Ordago]);
    }
}
