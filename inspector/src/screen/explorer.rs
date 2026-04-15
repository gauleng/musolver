use std::{collections::HashMap, fmt::Display, iter::zip, str::FromStr};

use iced::{
    Color, Element,
    Length::{self, Fill},
    Pixels, Point, Renderer, Size, Theme,
    alignment::{
        Horizontal,
        Vertical::{self, Top},
    },
    mouse,
    widget::{
        Canvas, Column, Container, Row,
        canvas::{self, Stroke, Text},
        column, pick_list, row, scrollable, text,
    },
};
use itertools::Itertools;
use musolver::{
    Game,
    mus::{Accion, Baraja, DistribucionCartaIter, Lance, Mano, RankingManos},
    solver::{
        AbstractChica, AbstractGrande, AbstractJuego, AbstractJugada, AbstractPares, AbstractPunto,
        GameType, HandConfiguration, InfoSet, LanceGame, MusGame, MusGameTwoHands,
        MusGameTwoPlayers, Strategy,
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    OneHand = 0,
    TwoHands = 1,
}

pub struct ActionPath {
    pub strategy: Strategy,
    pub buckets: Buckets,

    pub selected_tantos_mano: Option<u8>,
    pub tantos_mano: Vec<u8>,
    pub selected_tantos_postre: Option<u8>,
    pub tantos_postre: Vec<u8>,
    pub selected_strategy: Option<HandConfiguration>,
    pub strategies: Vec<HandConfiguration>,
    pub selected_actions: Vec<Option<OptionalAction>>,
    pub actions: Vec<(Lance, u8, Vec<OptionalAction>)>,
    pub view_mode: ViewMode,
    pub one_hand_squares: Vec<(AbstractJugada, SquareData<ExplorerEvent>)>,
    pub two_hands_squares: Vec<Vec<SquareData<ExplorerEvent>>>,
    pub hovered_square: Option<usize>,
    pub jugadas_pares: Vec<HayJugada>,
    pub jugadas_juego: Vec<HayJugada>,
    pub selected_pares: Option<HayJugada>,
    pub selected_juego: Option<HayJugada>,
}

impl ActionPath {
    pub fn new(strategy: Strategy) -> Self {
        let game_type = strategy.strategy_config.game_config.game_type;
        let strategies = match game_type {
            GameType::LanceGame(lance) | GameType::LanceGameTwoHands(lance) => match lance {
                Lance::Grande | Lance::Chica | Lance::Punto => vec![HandConfiguration::CuatroManos],
                _ => vec![
                    HandConfiguration::DosManos,
                    HandConfiguration::TresManos1vs2,
                    HandConfiguration::TresManos1vs2Intermedio,
                    HandConfiguration::TresManos2vs1,
                    HandConfiguration::CuatroManos,
                ],
            },
            _ => vec![],
        };

        let (jugadas_pares, selected_pares) = match game_type {
            GameType::MusGameTwoPlayers => (
                vec![
                    HayJugada::TwoPlayers([false, false]),
                    HayJugada::TwoPlayers([true, false]),
                    HayJugada::TwoPlayers([false, true]),
                    HayJugada::TwoPlayers([true, true]),
                ],
                Some(HayJugada::TwoPlayers([true, true])),
            ),
            GameType::MusGame => (
                vec![
                    HayJugada::FourPlayers([true, true, true, true]),
                    HayJugada::FourPlayers([true, true, true, false]),
                    HayJugada::FourPlayers([true, true, false, true]),
                    HayJugada::FourPlayers([true, false, true, true]),
                    HayJugada::FourPlayers([false, true, true, true]),
                    HayJugada::FourPlayers([true, true, false, false]),
                    HayJugada::FourPlayers([true, false, true, false]),
                    HayJugada::FourPlayers([true, false, false, true]),
                    HayJugada::FourPlayers([false, true, true, false]),
                    HayJugada::FourPlayers([false, true, false, true]),
                    HayJugada::FourPlayers([false, false, true, true]),
                    HayJugada::FourPlayers([true, false, false, false]),
                    HayJugada::FourPlayers([false, true, false, false]),
                    HayJugada::FourPlayers([false, false, true, false]),
                    HayJugada::FourPlayers([false, false, false, true]),
                    HayJugada::FourPlayers([false, false, false, false]),
                ],
                Some(HayJugada::FourPlayers([true, true, true, true])),
            ),
            _ => todo!(),
        };
        let (jugadas_juego, selected_juego) = match game_type {
            GameType::MusGameTwoPlayers => (
                vec![
                    HayJugada::TwoPlayers([false, false]),
                    HayJugada::TwoPlayers([true, true]),
                ],
                Some(HayJugada::TwoPlayers([true, true])),
            ),
            GameType::MusGame => (
                vec![
                    HayJugada::FourPlayers([true, true, true, true]),
                    HayJugada::FourPlayers([true, true, true, false]),
                    HayJugada::FourPlayers([true, true, false, true]),
                    HayJugada::FourPlayers([true, false, true, true]),
                    HayJugada::FourPlayers([false, true, true, true]),
                    HayJugada::FourPlayers([true, true, false, false]),
                    HayJugada::FourPlayers([true, false, false, true]),
                    HayJugada::FourPlayers([false, true, true, false]),
                    HayJugada::FourPlayers([false, false, true, true]),
                    HayJugada::FourPlayers([false, false, false, false]),
                ],
                Some(HayJugada::FourPlayers([true, true, true, true])),
            ),
            _ => todo!(),
        };

        let mut action_path = Self {
            one_hand_squares: vec![],
            two_hands_squares: vec![],
            buckets: Buckets::new(&Lance::Grande, None),
            view_mode: match game_type {
                GameType::MusGameTwoHands => ViewMode::TwoHands,
                _ => ViewMode::OneHand,
            },
            strategy: strategy.to_owned(),
            selected_tantos_mano: Some(0),
            tantos_mano: Vec::from_iter(0..40),
            selected_tantos_postre: Some(0),
            tantos_postre: Vec::from_iter(0..40),
            selected_actions: vec![],
            actions: vec![],
            selected_strategy: Some(HandConfiguration::CuatroManos),
            strategies,
            hovered_square: None,
            jugadas_pares,
            jugadas_juego,
            selected_pares,
            selected_juego,
        };
        let (lance, turn, actions) = action_path.game_state();
        if let musolver::NodeType::Player(player) = turn {
            action_path.append_action_picklists(lance, player as u8, &actions);
        }
        action_path.update_squares();
        action_path
    }

    fn append_action_picklists(&mut self, lance: Lance, player: u8, actions: &[Accion]) {
        let mut valores: Vec<OptionalAction> = vec![OptionalAction(None)];
        valores.extend(actions.iter().map(|c| OptionalAction(Some(*c))));
        self.selected_actions.push(None);
        self.actions.push((lance, player, valores));
    }

    fn strategy_node(
        &self,
        turno: u8,
        mano1: &Mano,
        mano2: Option<&Mano>,
    ) -> Option<(Vec<Accion>, Vec<f64>)> {
        let manos = self.selected_example_hands();
        let game_type = self.strategy.strategy_config.game_config.game_type;
        let history: Vec<Accion> = self.selected_history();
        let abstract_game = self.strategy.strategy_config.game_config.abstract_game;
        let tantos = [
            self.selected_tantos_mano.unwrap_or_default(),
            self.selected_tantos_postre.unwrap_or_default(),
        ];
        match game_type {
            GameType::LanceGame(lance) | GameType::LanceGameTwoHands(lance) => {
                let tipo_estrategia = self.selected_strategy.unwrap();
                let abstract_game_lance = if abstract_game { Some(lance) } else { None };
                let mut lance_game = LanceGame::new(lance, tantos, abstract_game);
                lance_game.new_with_configuration(tipo_estrategia);
                for action in &history {
                    lance_game.act(*action);
                }
                let info_set = InfoSet::str(
                    &tipo_estrategia,
                    &tantos,
                    mano1,
                    mano2,
                    &[],
                    abstract_game_lance,
                );
                Some(lance_game.actions()).zip(
                    self.strategy
                        .nodes
                        .get(&(info_set + &lance_game.history_str()))
                        .cloned(),
                )
            }
            GameType::MusGame => {
                let mut mus_game = MusGame::new(tantos, abstract_game);
                let info_set = format!("{}:{},{},", tantos[0], tantos[1], mano1);
                mus_game.new_random();
                for action in &history {
                    mus_game.act(*action);
                }
                Some(mus_game.actions()).zip(
                    self.strategy
                        .nodes
                        .get(&(info_set + &mus_game.history_str()))
                        .cloned(),
                )
            }
            GameType::MusGameTwoHands => {
                let mut mus_game = MusGameTwoHands::new(tantos, abstract_game);
                let info_set = format!("{}:{},{},{},", tantos[0], tantos[1], mano1, mano2.unwrap());
                mus_game.new_random();
                for action in &history {
                    mus_game.act(*action);
                }
                Some(mus_game.actions()).zip(
                    self.strategy
                        .nodes
                        .get(&(info_set + &mus_game.history_str()))
                        .cloned(),
                )
            }
            GameType::MusGameTwoPlayers => {
                let mut manos = [manos[0].clone(), manos[1].clone()];
                manos[turno as usize] = mano1.clone();
                let mut mus_game = MusGameTwoPlayers::new_with_hands(&manos, tantos, abstract_game);
                for action in &history {
                    mus_game.act(*action);
                }
                let info_set = mus_game.info_set_str(turno as usize);
                Some(mus_game.actions()).zip(self.strategy.nodes.get(&info_set).cloned())
            }
        }
    }

    fn selected_history(&self) -> Vec<Accion> {
        self.selected_actions
            .iter()
            .filter_map(|optional_action| optional_action.as_ref())
            .map(|optional_action| optional_action.0.unwrap())
            .collect()
    }

    pub fn update(&mut self, message: ExplorerEvent) {
        self.hovered_square = None;
        match message {
            ExplorerEvent::SetAction(level, action) => {
                self.selected_actions[level] = action.0.map(|_| action);
                self.selected_actions.drain(level + 1..);
                self.actions.drain(level + 1..);
            }
            ExplorerEvent::SetStrategy(strategy) => {
                self.selected_strategy = Some(strategy);
            }
            ExplorerEvent::SetTantosMano(tantos) => self.selected_tantos_mano = Some(tantos),
            ExplorerEvent::SetTantosPostre(tantos) => self.selected_tantos_postre = Some(tantos),
            ExplorerEvent::SelectBucket(bucket_id) => {
                self.hovered_square = bucket_id;
                return;
            }
            ExplorerEvent::SetPares(hay_jugada) => self.selected_pares = Some(hay_jugada),
            ExplorerEvent::SetJuego(hay_jugada) => self.selected_juego = Some(hay_jugada),
        }
        if let Some(None) = self.selected_actions.last() {
            self.selected_actions.pop();
            self.actions.pop();
        }

        self.update_squares();
        // let strategy_node = match game_type {
        //     GameType::LanceGame(_) | GameType::MusGame | GameType::MusGameTwoPlayers => {
        //         self.strategy_node(mano, None)
        //     }
        //     GameType::LanceGameTwoHands(_) => todo!(),
        //     GameType::MusGameTwoHands => self.strategy_node(mano, Some(mano)),
        // };
        // if let Some((actions, _)) = strategy_node {
        //     self.append_action_picklists(&actions);
        // } else {
        //     self.actions.clear();
        //     self.selected_actions.clear();
        // }
        // let turn = self.selected_action_node().to_play();
        // self.view_mode = match self.selected_strategy {
        //     Some(HandConfiguration::DosManos) => ViewMode::OneHand,
        //     Some(HandConfiguration::CuatroManos) => ViewMode::TwoHands,
        //     Some(HandConfiguration::TresManos1vs2)
        //     | Some(HandConfiguration::TresManos1vs2Intermedio) => {
        //         if turn.unwrap() == 0 {
        //             ViewMode::OneHand
        //         } else {
        //             ViewMode::TwoHands
        //         }
        //     }
        //     Some(HandConfiguration::TresManos2vs1) => {
        //         if turn.unwrap() == 0 {
        //             ViewMode::TwoHands
        //         } else {
        //             ViewMode::OneHand
        //         }
        //     }
        //     Some(HandConfiguration::SinLance) | None => ViewMode::OneHand,
        // };
    }

    fn update_squares(&mut self) {
        let (lance, turn, actions) = self.game_state();
        if let musolver::NodeType::Player(player) = turn {
            let has_pares = if lance == Lance::Juego || lance == Lance::Punto {
                match self.selected_pares {
                    Some(HayJugada::TwoPlayers(v)) => Some(v[player]),
                    Some(HayJugada::FourPlayers(v)) => Some(v[player]),
                    _ => None,
                }
            } else {
                None
            };
            self.buckets = Buckets::new(&lance, has_pares);

            match self.view_mode {
                ViewMode::OneHand => {
                    self.update_squares_one_hand(player as u8);
                }
                ViewMode::TwoHands => {
                    self.update_squares_two_hands(player as u8);
                }
            }

            self.append_action_picklists(lance, player as u8, &actions);
        }
    }

    fn update_squares_two_hands(&mut self, player: u8) {
        let avg_probability = |probabilities: Vec<(Vec<_>, Vec<_>)>| {
            let n_hands = probabilities.len();
            if n_hands > 0 {
                let actions = probabilities[0].0.clone();
                let n_actions = actions.len();
                let avg_probability =
                    probabilities
                        .into_iter()
                        .fold(vec![0.; n_actions], |avg, v| {
                            zip(avg, &v.1)
                                .map(|(a, v)| a + v / n_hands as f64)
                                .collect()
                        });
                Some((actions, avg_probability))
            } else {
                None
            }
        };
        let n_jugadas = self.buckets.jugadas().len();
        let mut two_hands_squares = Vec::with_capacity(n_jugadas);
        let mut bucket_id = 0;
        for jugada in self.buckets.jugadas() {
            let Some(_) = self.buckets.hands(jugada) else {
                continue;
            };
            let mut row = Vec::with_capacity(n_jugadas);
            for jugada2 in self.buckets.jugadas() {
                row.push(
                    SquareData::new(format!("{},{}", jugada, jugada2))
                        .on_hover(move || ExplorerEvent::SelectBucket(Some(bucket_id))),
                );
                bucket_id += 1;
            }
            two_hands_squares.push(row);
        }
        self.two_hands_squares = two_hands_squares;
        for column in 0..self.buckets.jugadas().len() {
            for row in 0..self.buckets.jugadas().len() {
                let jugada1 = &self.buckets.jugadas()[row];
                let jugada2 = &self.buckets.jugadas()[column];
                let (manos1, manos2) = self
                    .buckets
                    .hands(jugada1)
                    .zip(self.buckets.hands(jugada2))
                    .unwrap();
                let probabilities: Vec<(Vec<Accion>, Vec<f64>)> = manos1
                    .iter()
                    .cartesian_product(manos2.iter())
                    .filter_map(|(hand1, hand2)| self.strategy_node(player, hand1, Some(hand2)))
                    .collect();
                let square = &mut self.two_hands_squares[row][column];
                match avg_probability(probabilities) {
                    Some((actions, avg_probability)) => {
                        square.update_with_node(&actions, &avg_probability);
                        square.mano = format!("{jugada1},{jugada2}");
                    }
                    None => square.reset_probabilities(),
                }
            }
        }
    }

    fn update_squares_one_hand(&mut self, player: u8) {
        let n_jugadas = self.buckets.jugadas().len();
        let mut one_hand_squares = Vec::with_capacity(n_jugadas);
        let mut bucket_id = 0;
        for jugada in self.buckets.jugadas() {
            let Some(hands) = self.buckets.hands(jugada) else {
                continue;
            };
            one_hand_squares.extend(hands.iter().map(|hand| {
                let mut square_data = SquareData::new(hand.to_string())
                    .on_hover(move || ExplorerEvent::SelectBucket(Some(bucket_id)));
                if let Some((actions, probabilities)) = self.strategy_node(player, hand, None) {
                    square_data.update_with_node(&actions, &probabilities);
                }
                let square = (jugada.to_owned(), square_data);
                bucket_id += 1;
                square
            }));
        }

        self.one_hand_squares = one_hand_squares;
    }

    pub fn view(&self) -> Element<'_, ExplorerEvent> {
        let top_row = match self.strategy.strategy_config.game_config.game_type {
            GameType::LanceGame(_) | GameType::LanceGameTwoHands(_) => self.nav_bar_lance_game(),
            _ => self.nav_bar_mus_game(),
        };

        let legend =
            Container::new(Canvas::new(Legend::default()).width(700).height(60)).padding(20);

        let bucket_info = self
            .hovered_square
            .map_or_else(
                || column![text("-")],
                |bucket_id| {
                    //let jugada = self.one_hand_squares[bucket_id];
                    //let bucket_probability = 100. * self.buckets.probability(&jugada).unwrap();
                    column![
                        // text!(
                        //     "{} ({bucket_probability:.1}%)",
                        //     self.one_hand_squares[bucket_id].1.mano
                        // )
                        // .center()
                        // .size(20),
                        text!(
                            "Paso: {:.1}%",
                            self.one_hand_squares[bucket_id].1.paso * 100.
                        ),
                        text!(
                            "Quiero: {:.1}%",
                            self.one_hand_squares[bucket_id].1.quiero * 100.
                        ),
                        text!(
                            "Envido 2: {:.1}%",
                            self.one_hand_squares[bucket_id].1.envido2 * 100.
                        ),
                        text!(
                            "Envido 5: {:.1}%",
                            self.one_hand_squares[bucket_id].1.envido5 * 100.
                        ),
                        text!(
                            "Envido 10: {:.1}%",
                            self.one_hand_squares[bucket_id].1.envido10 * 100.
                        ),
                        text!(
                            "Órdago: {:.1}%",
                            self.one_hand_squares[bucket_id].1.ordago * 100.
                        ),
                    ]
                },
            )
            .width(300)
            .padding(20);

        let mut matrix = Column::new();
        if self.view_mode == ViewMode::OneHand {
            let squares = column(
                self.one_hand_squares
                    .chunk_by(|a, b| a.0 == b.0)
                    .map(|chunk| {
                        chunk
                            .iter()
                            .map(|(_, square)| Canvas::new(square).width(50).height(50))
                            .map(Element::from)
                    })
                    .map(row)
                    .map(Element::from),
            );
            matrix = matrix.push(squares);
        } else {
            for square_column in &self.two_hands_squares {
                let mut row = Row::new();
                for square_row in square_column {
                    row = row.push(Canvas::new(square_row).width(50).height(50));
                }
                matrix = matrix.push(row);
            }
        }

        let scrollable_matrix = row![
            bucket_info,
            scrollable(matrix)
                .direction(scrollable::Direction::Both {
                    vertical: scrollable::Scrollbar::default(),
                    horizontal: scrollable::Scrollbar::default(),
                })
                .width(Length::Fill)
        ];
        let layout = column![top_row, legend, scrollable_matrix].align_x(Horizontal::Center);

        layout.into()
    }

    fn nav_bar_lance_game(&self) -> Row<'_, ExplorerEvent> {
        let mut top_row = Row::new();

        let pick_strategy = pick_list(
            &self.strategies[..],
            self.selected_strategy,
            ExplorerEvent::SetStrategy,
        )
        .placeholder("Select a strategy");
        top_row = top_row.push(pick_strategy);

        let pick_tantos_mano = pick_list(
            &self.tantos_mano[..],
            self.selected_tantos_mano,
            ExplorerEvent::SetTantosMano,
        );
        top_row = top_row.push(pick_tantos_mano);

        let pick_tantos_postre = pick_list(
            &self.tantos_postre[..],
            self.selected_tantos_postre,
            ExplorerEvent::SetTantosPostre,
        );
        top_row = top_row.push(pick_tantos_postre);

        for level in 0..self.selected_actions.len() {
            let pick_action_n = pick_list(
                &self.actions[level].2[..],
                self.selected_actions[level],
                move |elem| ExplorerEvent::SetAction(level, elem),
            )
            .placeholder("Select an action");
            top_row = top_row.push(pick_action_n);
        }
        top_row = top_row.width(Fill).align_y(Top).spacing(10);
        top_row
    }

    fn nav_bar_mus_game(&self) -> Row<'_, ExplorerEvent> {
        let mut top_row = Row::new();

        let pick_tantos_mano = column![
            text("Tantos mano").size(14),
            pick_list(
                &self.tantos_mano[..],
                self.selected_tantos_mano,
                ExplorerEvent::SetTantosMano,
            )
        ];
        top_row = top_row.push(pick_tantos_mano);

        let pick_tantos_postre = column![
            text("Tantos postre").size(14),
            pick_list(
                &self.tantos_postre[..],
                self.selected_tantos_postre,
                ExplorerEvent::SetTantosPostre,
            )
        ];
        top_row = top_row.push(pick_tantos_postre);

        let pick_pares = column![
            text("Pares").size(14),
            pick_list(
                &self.jugadas_pares[..],
                self.selected_pares,
                ExplorerEvent::SetPares,
            )
        ];
        top_row = top_row.push(pick_pares);

        let pick_juego = column![
            text("Juego").size(14),
            pick_list(
                &self.jugadas_juego[..],
                self.selected_juego,
                ExplorerEvent::SetJuego,
            )
        ];
        top_row = top_row.push(pick_juego);

        let mut level = 0;
        let picklists = row(self
            .actions
            .chunk_by(|a, b| a.0 == b.0)
            .map(|chunk| {
                column(
                    std::iter::once(format!("{:?}", chunk[0].0))
                        .map(text)
                        .map(Element::from)
                        .chain(
                            chunk
                                .iter()
                                .map(|(_, _, actions)| {
                                    let picklist = pick_list(
                                        &actions[..],
                                        self.selected_actions[level],
                                        move |elem| ExplorerEvent::SetAction(level, elem),
                                    )
                                    .placeholder("Select an action");
                                    level += 1;
                                    picklist
                                })
                                .map(Element::from),
                        ),
                )
            })
            .map(Element::from));
        top_row = top_row.push(picklists);
        top_row = top_row.width(Fill).align_y(Top).spacing(10);
        top_row
    }

    fn game_state(&self) -> (Lance, musolver::NodeType, Vec<Accion>) {
        let manos = self.selected_example_hands();
        let mut game = MusGameTwoPlayers::new_with_hands(
            &[manos[0].clone(), manos[1].clone()],
            [
                self.selected_tantos_mano.unwrap(),
                self.selected_tantos_postre.unwrap(),
            ],
            self.strategy.strategy_config.game_config.abstract_game,
        );
        self.selected_history()
            .into_iter()
            .for_each(|action| game.act(action));
        let mus_game = game.mus_game();
        let lance = mus_game.unwrap().lance_actual().unwrap();
        let turno = game.current_player();
        let actions = game.actions();
        (lance, turno, actions)
    }

    fn example_hand(pares: bool, juego: bool) -> Mano {
        match (pares, juego) {
            (false, false) => Mano::from_str("6541").unwrap(),
            (true, false) => Mano::from_str("1111").unwrap(),
            (false, true) => Mano::from_str("RCS1").unwrap(),
            (true, true) => Mano::from_str("RRRR").unwrap(),
        }
    }

    fn selected_example_hands(&self) -> Vec<Mano> {
        let (pares, juego) = (self.selected_pares, self.selected_juego);
        match (pares, juego) {
            (Some(HayJugada::TwoPlayers([p1, p2])), Some(HayJugada::TwoPlayers([j1, j2]))) => {
                vec![Self::example_hand(p1, j1), Self::example_hand(p2, j2)]
            }
            (
                Some(HayJugada::FourPlayers([p1, p2, p3, p4])),
                Some(HayJugada::FourPlayers([j1, j2, j3, j4])),
            ) => {
                vec![
                    Self::example_hand(p1, j1),
                    Self::example_hand(p2, j2),
                    Self::example_hand(p3, j3),
                    Self::example_hand(p4, j4),
                ]
            }
            _ => {
                vec![]
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ExplorerEvent {
    SetAction(usize, OptionalAction),
    SetStrategy(HandConfiguration),
    SetTantosMano(u8),
    SetTantosPostre(u8),
    SelectBucket(Option<usize>),
    SetPares(HayJugada),
    SetJuego(HayJugada),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OptionalAction(Option<Accion>);

impl Display for OptionalAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(a) => write!(f, "{}", a),
            None => write!(f, ""),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HayJugada {
    TwoPlayers([bool; 2]),
    FourPlayers([bool; 4]),
}

impl Display for HayJugada {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TwoPlayers([a, b]) => {
                write!(f, "{}{}", if *a { 1 } else { 0 }, if *b { 1 } else { 0 })
            }
            Self::FourPlayers([a, b, c, d]) => {
                write!(
                    f,
                    "{}{}{}{}",
                    if *a { 1 } else { 0 },
                    if *b { 1 } else { 0 },
                    if *c { 1 } else { 0 },
                    if *d { 1 } else { 0 }
                )
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct Legend {}

impl Legend {
    pub fn legend_palette() -> [Color; 6] {
        [
            Color::parse("006E90").unwrap(),
            Color::parse("2F9332").unwrap(),
            Color::parse("FABC3F").unwrap(),
            Color::parse("E85C0D").unwrap(),
            Color::parse("C7253E").unwrap(),
            Color::parse("821131").unwrap(),
        ]
    }
}

impl<AppEvent> canvas::Program<AppEvent> for Legend {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let height = bounds.height;
        let width = bounds.width;

        let region_widths = [width / 6.; 6];
        let region_x_position: Vec<f32> = (0..6).map(|v| width * v as f32 / 6.).collect();
        let region_colors = Self::legend_palette();
        let region_text = [
            "Paso",
            "Quiero",
            "Envido 2",
            "Envido 5",
            "Envido 10",
            "Órdago",
        ];

        for i in 0..region_widths.len() {
            frame.fill_rectangle(
                Point::new(region_x_position[i], 0.),
                Size::new(width / 6., height),
                region_colors[i],
            );
            let mut text = Text {
                content: String::from(region_text[i]),
                position: Point::new(region_x_position[i] + 10., height / 2.0),
                color: theme.palette().text,
                ..Text::default()
            };
            text.vertical_alignment = Vertical::Center;
            frame.fill_text(text);
        }

        vec![frame.into_geometry()]
    }
}

pub struct SquareData<Message> {
    pub paso: f64,
    pub quiero: f64,
    pub envido2: f64,
    pub envido5: f64,
    pub envido10: f64,
    pub ordago: f64,
    pub resto: f64,
    pub mano: String,
    pub cache: canvas::Cache,
    on_hover: Option<Box<dyn Fn() -> Message + 'static>>,
}

impl<Message> SquareData<Message> {
    pub fn new(mano: String) -> Self {
        Self {
            paso: 0.,
            quiero: 0.,
            envido2: 0.,
            envido5: 0.,
            envido10: 0.,
            ordago: 0.,
            resto: 0.,
            mano,
            cache: canvas::Cache::default(),
            on_hover: None,
        }
    }

    pub fn on_hover(mut self, on_hover: impl Fn() -> Message + 'static) -> Self {
        self.on_hover = Some(Box::new(on_hover));
        self
    }
}

impl<Message> SquareData<Message> {
    pub fn update_with_node(&mut self, actions: &[Accion], probabilities: &[f64]) {
        self.reset_probabilities();
        self.cache.clear();
        actions
            .iter()
            .zip(probabilities.iter())
            .for_each(|(c, p)| match c {
                Accion::Paso => self.paso = *p,
                Accion::Envido(2) => self.envido2 = *p,
                Accion::Envido(5) => self.envido5 = *p,
                Accion::Envido(10) => self.envido10 = *p,
                Accion::Quiero => self.quiero = *p,
                Accion::Ordago => self.ordago = *p,
                _ => self.resto = *p,
            });
    }

    pub fn reset_probabilities(&mut self) {
        self.paso = 0.;
        self.quiero = 0.;
        self.envido2 = 0.;
        self.envido5 = 0.;
        self.envido10 = 0.;
        self.ordago = 0.;
        self.resto = 0.;
    }
}

impl<Message> canvas::Program<Message> for SquareData<Message> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let content = self.cache.draw(renderer, bounds.size(), |frame| {
            let height = bounds.height;
            let width = bounds.width;
            let region_widths = [
                width * self.paso as f32,
                width * self.quiero as f32,
                width * self.envido2 as f32,
                width * self.envido5 as f32,
                width * self.envido10 as f32,
                width * self.ordago as f32,
            ];
            let region_colors = Legend::legend_palette();
            let region_x_position: Vec<f32> = region_widths
                .iter()
                .scan(0., |x_pos, width| {
                    let ret = Some(*x_pos);
                    *x_pos += width;
                    ret
                })
                .collect();
            for i in 0..region_widths.len() {
                let rect_quiero = canvas::Path::rectangle(
                    Point::new(region_x_position[i], 0.),
                    Size::new(region_widths[i], height),
                );
                frame.fill(&rect_quiero, region_colors[i]);
            }
            frame.stroke_rectangle(
                Point::ORIGIN,
                Size::new(width, height),
                Stroke::default().with_color(Color::BLACK).with_width(2.),
            );
            let mut text = iced::widget::canvas::Text {
                content: String::from(&self.mano),
                position: Point::new(width / 2.0, height / 2.0),
                color: theme.palette().text,
                ..iced::widget::canvas::Text::default()
            };
            text.vertical_alignment = Vertical::Center;
            text.horizontal_alignment = Horizontal::Center;
            text.size = Pixels(10.);
            frame.fill_text(text);
        });

        // Then, we produce the geometry
        vec![content]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: iced::Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<Message>) {
        if let canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            if cursor.position_in(bounds).is_some() {
                if let Some(callback) = &self.on_hover {
                    return (canvas::event::Status::Captured, Some(callback()));
                }
            }
        }
        (canvas::event::Status::Ignored, None)
    }
}

pub struct Buckets {
    buckets: HashMap<AbstractJugada, (Vec<Mano>, f64)>,
    jugadas: Vec<AbstractJugada>,
}

impl Buckets {
    pub fn new(lance: &Lance, has_pares: Option<bool>) -> Self {
        let one_hand_list = Self::one_hand_list(lance);
        let mut buckets = HashMap::new();
        one_hand_list
            .iter()
            .filter(|(hand, _)| has_pares.is_none_or(|required| hand.pares().is_some() == required))
            .filter_map(|(hand, probability)| match lance {
                Lance::Grande => Some((AbstractGrande::abstract_hand(hand), (hand, probability))),
                Lance::Chica => Some((AbstractChica::abstract_hand(hand), (hand, probability))),
                Lance::Pares => AbstractPares::abstract_hand(hand).zip(Some((hand, probability))),
                Lance::Juego => AbstractJuego::abstract_hand(hand).zip(Some((hand, probability))),
                Lance::Punto => Some((AbstractPunto::abstract_hand(hand), (hand, probability))),
            })
            .for_each(|(jugada, (hand, probability))| {
                let entry = buckets.entry(jugada).or_insert((vec![], 0.));
                entry.0.push(hand.to_owned());
                entry.1 += probability;
            });
        let mut jugadas: Vec<_> = buckets.keys().cloned().collect();
        jugadas.sort();

        Self { jugadas, buckets }
    }

    pub fn hands(&self, jugada: &AbstractJugada) -> Option<&Vec<Mano>> {
        self.buckets.get(jugada).map(|(hands, _)| hands)
    }

    pub fn probability(&self, jugada: &AbstractJugada) -> Option<&f64> {
        self.buckets.get(jugada).map(|(_, probability)| probability)
    }

    pub fn jugadas(&self) -> &Vec<AbstractJugada> {
        &self.jugadas
    }

    fn one_hand_list(lance: &Lance) -> Vec<(Mano, f64)> {
        let manos = DistribucionCartaIter::new(&Baraja::FREC_BARAJA_MUS, 4)
            .map(|(cards, prob)| (Mano::new(cards), prob));
        manos
            .filter(|(hand, _)| hand.jugada(lance).is_some())
            .sorted_by(|(a, _), (b, _)| lance.compara_manos(a, b))
            .collect()
    }
}
