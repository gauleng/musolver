use std::{fmt::Write, rc::Rc};

use arrayvec::ArrayString;
use itertools::Itertools;

use crate::{
    Game, NodeType,
    mus::{
        Accion, Apuesta, Baraja, Carta, CuatroJugadores, DistribucionCartaIter,
        DistribucionDobleCartaIter, DosJugadores, FaseEnvites, FasePartida, Lance, Mano,
        PartidaMus, Turno,
    },
    solver::ManosNormalizadas,
};

#[derive(Debug, Clone)]
pub struct MusGame {
    tantos: [u8; 2],
    partida: Option<FaseEnvites<CuatroJugadores>>,
    history_str: ArrayString<64>,
    info_set_prefix: [ArrayString<16>; 4],
    last_action: Option<Accion>,
    manos_pares: ArrayString<4>,
    manos_juego: ArrayString<4>,
    abstract_game: bool,
    utility_table: Option<Rc<[[f64; 40]; 40]>>,
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
            utility_table: None,
        }
    }

    pub fn with_hands(self, manos: [Mano; 4]) -> Self {
        let info_set_prefix = MusGame::info_set_prefix(
            &manos,
            &self.tantos,
            if self.abstract_game {
                Some(Lance::Grande)
            } else {
                None
            },
        );
        let (manos_pares, manos_juego) = MusGame::jugadas_manos(&manos);
        let partida = Some(FaseEnvites::<CuatroJugadores>::new(manos, self.tantos));
        let history_str = ArrayString::<64>::from("M").unwrap();
        Self {
            partida,
            history_str,
            info_set_prefix,
            manos_pares,
            manos_juego,
            ..self
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
    pub fn with_utility_table(self, utility_table: Rc<[[f64; 40]; 40]>) -> Self {
        Self {
            utility_table: Some(utility_table),
            ..self
        }
    }

    pub fn default_utility_table() -> [[f64; 40]; 40] {
        std::array::from_fn(|t1| std::array::from_fn(|t2| (t1 - t2) as f64))
    }
}

impl Game for MusGame {
    type Action = Accion;
    const N_PLAYERS: usize = 4;

    fn utility(&mut self, player: usize) -> f64 {
        let tantos = self.partida.as_mut().unwrap().tantos();

        if let Some(utility_table) = &self.utility_table {
            if tantos[0] == 40 || tantos[1] == 40 {
                let payoff = [
                    tantos[0] as i8 - tantos[1] as i8,
                    tantos[1] as i8 - tantos[0] as i8,
                ];

                payoff[player % 2] as f64
            } else {
                let expected_utility = utility_table[tantos[1] as usize][tantos[0] as usize];
                if player == 0 {
                    -expected_utility
                } else {
                    expected_utility
                }
            }
        } else {
            let payoff = [
                tantos[0] as i8 - tantos[1] as i8,
                tantos[1] as i8 - tantos[0] as i8,
            ];

            payoff[player % 2] as f64
        }
    }

    fn info_set_str(&self, player: usize) -> String {
        let mut output = String::with_capacity(15 + self.history_str.len());
        output.push_str(&self.info_set_prefix[player]);
        output.push_str(&self.history_str());
        output
    }

    fn new_random(&mut self) {
        let mut baraja = Baraja::baraja_mus();
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
        let partida = FaseEnvites::<CuatroJugadores>::new(manos, self.tantos);
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
                            Self::new(self.tantos, self.abstract_game).with_hands(manos),
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
    partida: Option<FaseEnvites<CuatroJugadores>>,
    history_str: ArrayString<64>,
    info_set_prefix: [ArrayString<16>; 2],
    manos_pares: ArrayString<4>,
    manos_juego: ArrayString<4>,
    abstract_game: bool,
    utility_table: Option<Rc<[[f64; 40]; 40]>>,
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
            utility_table: None,
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
        let partida = Some(FaseEnvites::<CuatroJugadores>::new(manos, tantos));
        let history_str = ArrayString::<64>::from("M").unwrap();
        Self {
            partida,
            tantos,
            history_str,
            info_set_prefix,
            manos_pares,
            manos_juego,
            abstract_game,
            utility_table: None,
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
    pub fn with_utility_table(self, utility_table: Rc<[[f64; 40]; 40]>) -> Self {
        Self {
            utility_table: Some(utility_table),
            ..self
        }
    }
}

impl Game for MusGameTwoHands {
    type Action = Accion;
    const N_PLAYERS: usize = 2;

    fn utility(&mut self, player: usize) -> f64 {
        let tantos = self.partida.as_mut().unwrap().tantos();

        if let Some(utility_table) = &self.utility_table {
            if tantos[0] == 40 || tantos[1] == 40 {
                let payoff = [
                    tantos[0] as i8 - tantos[1] as i8,
                    tantos[1] as i8 - tantos[0] as i8,
                ];

                payoff[player % 2] as f64
            } else {
                let expected_utility = utility_table[tantos[1] as usize][tantos[0] as usize];
                if player == 0 {
                    -expected_utility
                } else {
                    expected_utility
                }
            }
        } else {
            let payoff = [
                tantos[0] as i8 - tantos[1] as i8,
                tantos[1] as i8 - tantos[0] as i8,
            ];

            payoff[player] as f64
        }
    }

    fn info_set_str(&self, player: usize) -> String {
        let mut output = String::with_capacity(15 + self.history_str.len());
        output.push_str(&self.info_set_prefix[player]);
        output.push_str(&self.history_str());
        output
    }

    fn new_random(&mut self) {
        let mut baraja = Baraja::baraja_mus();
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
        let partida = FaseEnvites::<CuatroJugadores>::new(manos, self.tantos);
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
    cards: Option<CardSource>,
    partida: Option<PartidaMus<DosJugadores>>,
    history_str: ArrayString<64>,
    info_set_prefix: [ArrayString<16>; 2],
    descarte_str: [ArrayString<5>; 2],
    manos_pares: ArrayString<2>,
    manos_juego: ArrayString<2>,
    hubo_mus: bool,
    abstract_game: bool,
    utility_table: Option<Rc<[[f64; 40]; 40]>>,
}

impl MusGameTwoPlayers {
    pub fn new(tantos: [u8; 2], abstract_game: bool) -> Self {
        Self {
            partida: None,
            cards: None,
            tantos,
            history_str: ArrayString::new(),
            info_set_prefix: [ArrayString::new(); 2],
            descarte_str: [ArrayString::new(); 2],
            manos_pares: ArrayString::new(),
            manos_juego: ArrayString::new(),
            hubo_mus: false,
            abstract_game,
            utility_table: None,
        }
    }

    pub fn with_utility_table(self, utility_table: Rc<[[f64; 40]; 40]>) -> Self {
        Self {
            utility_table: Some(utility_table),
            ..self
        }
    }

    pub fn with_hands(self, manos: [Mano; 2]) -> Self {
        let mut new_game = self.clone();
        new_game.set_hands(manos);
        new_game
    }

    fn set_hands(&mut self, manos: [Mano; 2]) {
        self.info_set_prefix = MusGameTwoPlayers::info_set_prefix(
            &manos,
            &self.tantos,
            self.abstract_game.then_some(Lance::Grande),
        );
        (self.manos_pares, self.manos_juego) = MusGameTwoPlayers::jugadas_manos(&manos);
        self.partida = Some(PartidaMus::<DosJugadores>::new(manos, self.tantos));
        self.history_str = ArrayString::<64>::from("M").unwrap();
    }

    fn set_card_source(&mut self, cartas: CardSource) {
        self.cards = Some(cartas);
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
        core::array::from_fn(|i| {
            let mut w = InfoSetWriter(ArrayString::<16>::new());
            w.tantos(tantos).mano(&manos[i], abstracto);
            w.into_inner()
        })
    }

    fn iter_descartes<const N: usize>(
        &self,
        estado_baraja: &[(Carta, u8); 8],
    ) -> Vec<(MusGameTwoPlayers, f64)> {
        let mut partidas = Vec::new();
        let mut iter = DistribucionCartaIter::<N>::new(estado_baraja);
        while let Some((nuevas, probability)) = iter.next() {
            let mut game = self.clone();
            game.partida
                .as_mut()
                .unwrap()
                .descartar_con_nuevas(&nuevas)
                .unwrap();
            let Some(CardSource::Iterable(dist)) = &mut game.cards else {
                unreachable!()
            };
            for (d, f) in std::iter::zip(dist, iter.current_frequencies()) {
                d.1 = *f as u8;
            }
            partidas.push((game, probability));
        }
        partidas
    }
}

impl Game for MusGameTwoPlayers {
    type Action = Accion;
    const N_PLAYERS: usize = 2;

    fn utility(&mut self, player: usize) -> f64 {
        let tantos = self.partida.as_mut().unwrap().tantos();

        if let Some(utility_table) = &self.utility_table {
            if tantos[0] == 40 || tantos[1] == 40 {
                let payoff = [
                    tantos[0] as i8 - tantos[1] as i8,
                    tantos[1] as i8 - tantos[0] as i8,
                ];

                payoff[player % 2] as f64
            } else {
                let expected_utility = utility_table[tantos[1] as usize][tantos[0] as usize];
                if player == 0 {
                    -expected_utility
                } else {
                    expected_utility
                }
            }
        } else {
            let payoff = [
                tantos[0] as i8 - tantos[1] as i8,
                tantos[1] as i8 - tantos[0] as i8,
            ];

            payoff[player] as f64
        }
    }

    fn info_set_str(&self, player: usize) -> String {
        let mut output = String::with_capacity(15 + self.history_str.len());
        output.push_str(&self.info_set_prefix[player]);
        output.push_str(&self.descarte_str[player]);
        output.push_str(&self.history_str());
        output
    }

    fn new_random(&mut self) {
        match &mut self.partida {
            None => {
                let mut baraja = Baraja::baraja_mus();
                let manos = baraja.repartir_manos();
                self.set_hands(manos);
                self.set_card_source(CardSource::Baraja(baraja));
            }
            Some(p) => {
                if let Some(CardSource::Baraja(baraja)) = &mut self.cards {
                    let turno = match p
                        .turno()
                        .expect("Some player must be active to call new_random() after game start")
                    {
                        Turno::Jugador(t) => t,
                        Turno::Pareja(_) => todo!(),
                    } as usize;
                    let descartes = p.descartadas().unwrap();
                    InfoSetWriter(&mut self.descarte_str[turno]).descarte(&descartes);
                    let nuevas = baraja.descartar(descartes.into_iter());
                    let _ = p.descartar_con_nuevas(&nuevas);
                    self.history_str.push('C');
                }
            }
        }
    }

    fn reset(&mut self) {
        self.partida = None;
        self.history_str.clear();
        self.info_set_prefix = [ArrayString::new(); 2];
        self.manos_pares.clear();
        self.manos_juego.clear();
        self.cards = None;
        self.hubo_mus = false;
        self.descarte_str = [ArrayString::new(); 2];
    }

    fn new_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        match &self.partida {
            None => {
                let mut partidas = Vec::new();
                let mut iter = DistribucionDobleCartaIter::new(&Baraja::FREC_BARAJA_MUS);
                while let Some((mano1, mano2, probability)) = iter.next() {
                    let mut game = MusGameTwoPlayers::new(self.tantos, self.abstract_game)
                        .with_hands([Mano::new(mano1), Mano::new(mano2)]);
                    let mut dist = Baraja::FREC_BARAJA_MUS;
                    let freq = iter.current_frequencies();
                    for (d, f) in std::iter::zip(&mut dist, freq) {
                        d.1 = *f as u8;
                    }
                    game.set_card_source(CardSource::Iterable(dist));
                    partidas.push((game, probability));
                }
                partidas.into_iter()
            }
            Some(p) => {
                let Some(CardSource::Iterable(estado_baraja)) = self.cards else {
                    todo!()
                };
                let turno = match p
                    .turno()
                    .expect("Some player must be active to call new_iter() after game started")
                {
                    Turno::Jugador(t) => t,
                    Turno::Pareja(_) => todo!(),
                } as usize;
                let mut game = self.clone();
                let descartes = game.partida.as_ref().unwrap().descartadas().unwrap();
                InfoSetWriter(&mut game.descarte_str[turno]).descarte(&descartes);
                game.history_str.push('C');
                match descartes.len() {
                    1 => game.iter_descartes::<1>(&estado_baraja),
                    2 => game.iter_descartes::<2>(&estado_baraja),
                    3 => game.iter_descartes::<3>(&estado_baraja),
                    4 => game.iter_descartes::<4>(&estado_baraja),
                    _ => unreachable!(),
                }
                .into_iter()
            }
        }
    }

    fn actions(&self) -> Vec<Accion> {
        let partida = self.partida.as_ref().unwrap();
        match partida.fase() {
            Some(FasePartida::Mus) => {
                if self.hubo_mus {
                    vec![Accion::NoMus]
                } else {
                    vec![Accion::Mus, Accion::NoMus]
                }
            }
            Some(FasePartida::Descartes) => {
                let turno = match partida
                    .turno()
                    .expect("Some player must be active to call actions()")
                {
                    Turno::Jugador(t) => t,
                    Turno::Pareja(_) => todo!(),
                } as usize;
                let mano = &partida.manos()[turno];
                let mut descartes = [false; 4];
                for (idx, carta) in mano.iter().enumerate() {
                    descartes[idx] = *carta != Carta::Rey;
                }
                if descartes == [false; 4] {
                    descartes[0] = true;
                }
                vec![Accion::Descartar(descartes)]
            }
            Some(FasePartida::Envites(_)) => {
                let fase_envites = partida.fase_envites().unwrap();
                let ultimo_envite: Apuesta = fase_envites.ultima_apuesta();
                let apuesta_maxima = fase_envites.apuesta_maxima();
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
            Some(FasePartida::DescartePendiente) => vec![],
            None => todo!(),
        }
    }

    fn current_player(&self) -> NodeType {
        match &self.partida {
            None => NodeType::Chance,
            Some(partida) => match partida.fase() {
                None => NodeType::Terminal,
                Some(FasePartida::DescartePendiente) => NodeType::Chance,
                Some(FasePartida::Mus | FasePartida::Descartes | FasePartida::Envites(_)) => {
                    match partida.turno() {
                        Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                            NodeType::Player(player_id as usize % 2)
                        }
                        None => NodeType::Terminal,
                    }
                }
            },
        }
    }

    fn act(&mut self, a: Accion) {
        self.history_str.push_str(&a.to_string());
        if let Some(partida) = self.partida.as_mut() {
            match &partida.fase() {
                Some(FasePartida::Descartes) => {
                    let _ = partida.actuar(a);
                    self.hubo_mus = true;
                }
                Some(FasePartida::Mus) => {
                    let _ = partida.actuar(a);
                    if let Some(FasePartida::Envites(lance)) = &partida.fase() {
                        let manos = partida.manos();
                        self.info_set_prefix = MusGameTwoPlayers::info_set_prefix(
                            manos,
                            &self.tantos,
                            self.abstract_game.then_some(*lance),
                        );
                        (self.manos_pares, self.manos_juego) =
                            MusGameTwoPlayers::jugadas_manos(manos);
                    }
                }
                Some(FasePartida::Envites(lance_previo)) => {
                    let _ = partida.actuar(a);
                    let Some(FasePartida::Envites(lance_siguiente)) = &partida.fase() else {
                        return;
                    };
                    if lance_previo != lance_siguiente {
                        self.info_set_prefix = MusGameTwoPlayers::info_set_prefix(
                            partida.manos(),
                            &self.tantos,
                            self.abstract_game.then_some(*lance_siguiente),
                        );
                        match lance_siguiente {
                            Lance::Pares => self.history_str.push_str(self.manos_pares.as_str()),
                            Lance::Punto | Lance::Juego => {
                                if lance_previo != &Lance::Pares {
                                    self.history_str.push_str(self.manos_pares.as_str());
                                }
                                self.history_str.push_str(self.manos_juego.as_str());
                            }
                            _ => {}
                        }
                    }
                }
                _ => todo!(),
            }
        }
    }

    fn history_str(&self) -> String {
        self.history_str.to_string()
    }
}

#[derive(Debug, Clone)]
enum CardSource {
    Baraja(Baraja),
    Iterable([(Carta, u8); 8]),
}

struct InfoSetWriter<W: Write>(W);

impl<W: Write> InfoSetWriter<W> {
    fn tantos(&mut self, tantos: &[u8; 2]) -> &mut Self {
        let _ = write!(self.0, "{}:{},", tantos[0], tantos[1]);
        self
    }

    fn mano(&mut self, mano: &Mano, abstracto: Option<Lance>) -> &mut Self {
        if let Some(lance) = abstracto {
            let mano_abstracta = ManosNormalizadas::mano_to_abstract_string(mano, &lance);
            let _ = write!(self.0, "{},", mano_abstracta);

            self
        } else {
            let _ = write!(self.0, "{},", mano);

            self
        }
    }

    fn descarte(&mut self, descartes: &[Carta]) -> &mut Self {
        for d in descartes {
            let _ = write!(self.0, "{}", char::from(d));
        }
        let _ = write!(self.0, ",");

        self
    }

    fn into_inner(self) -> W {
        self.0
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
        let mut game = MusGameTwoPlayers::new([38, 37], false).with_hands(manos);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,M");
        game.act(Accion::NoMus);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mn");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(1), "38:37,RCC1,Mnp");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpp");
        game.act(Accion::Paso);
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpppo");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpppop11");
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(1), "38:37,RCC1,Mnpppop11o");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpppop11op11");

        let manos = [
            Mano::from_str("R775").unwrap(),
            Mano::from_str("S651").unwrap(),
        ];
        let mut game = MusGameTwoPlayers::new([38, 37], false).with_hands(manos);
        game.act(Accion::NoMus);
        game.act(Accion::Paso);
        game.act(Accion::Paso);

        game.act(Accion::Paso);
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(0), "38:37,R775,Mnpppo");
        game.act(Accion::Paso);

        assert_eq!(game.info_set_str(0), "38:37,R775,Mnpppop1000");
    }

    #[test]
    fn two_players_actions() {
        let manos = [
            Mano::from_str("RRR5").unwrap(),
            Mano::from_str("RCC1").unwrap(),
        ];
        let mut game = MusGameTwoPlayers::new([35, 35], false).with_hands(manos.clone());
        game.act(Accion::NoMus);
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
        let mut game = MusGameTwoPlayers::new([37, 37], false).with_hands(manos.clone());
        game.act(Accion::NoMus);
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

    #[test]
    fn update_hands_after_discard() {
        // Ninguna mano tiene juego al reparto inicial.
        let manos = [
            Mano::from_str("4411").unwrap(),
            Mano::from_str("5511").unwrap(),
        ];
        let mut game = MusGameTwoPlayers::new([0, 0], false).with_hands(manos);
        assert_eq!(game.manos_juego.as_str(), "00");

        game.act(Accion::Mus);
        game.act(Accion::Mus);

        game.act(Accion::Descartar([true, true, true, false]));
        let _ = game.partida.as_mut().unwrap().descartar_con_nuevas(&[
            Carta::Rey,
            Carta::Rey,
            Carta::Rey,
        ]);
        game.act(Accion::Descartar([true, false, false, false]));
        let _ = game
            .partida
            .as_mut()
            .unwrap()
            .descartar_con_nuevas(&[Carta::As]);

        game.act(Accion::NoMus);

        let manos_reales = game.partida.as_ref().unwrap().manos().clone();
        assert!(manos_reales[0].hay_juego());
        assert!(!manos_reales[1].hay_juego());

        // El flag de juego debe reflejar las manos tras el descarte, no el reparto inicial.
        assert_eq!(game.manos_juego.as_str(), "10");
        let prefijo_esperado =
            MusGameTwoPlayers::info_set_prefix(&manos_reales, &game.tantos, None);
        assert_eq!(game.info_set_prefix, prefijo_esperado);
    }
}
