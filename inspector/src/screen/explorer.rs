use std::{collections::HashMap, fmt::Display, iter::zip};

use iced::{
    alignment::{
        Horizontal,
        Vertical::{self, Top},
    },
    mouse,
    widget::{
        canvas::{self, Stroke, Text},
        column, pick_list, row, scrollable, text, Canvas, Column, Container, Row,
    },
    Color, Element,
    Length::{self, Fill},
    Pixels, Point, Renderer, Size, Theme,
};
use itertools::Itertools;
use musolver::{
    mus::{Accion, Carta, CartaIter, Lance, Mano, RankingManos},
    solver::{
        AbstractChica, AbstractGrande, AbstractJuego, AbstractJugada, AbstractPares, AbstractPunto,
        HandConfiguration, InfoSet, LanceGame, Strategy,
    },
    Game,
};

#[derive(Clone, Debug)]
pub enum ExplorerEvent {
    SetAction(usize, OptionalAction),
    SetStrategy(HandConfiguration),
    SetTantosMano(u8),
    SetTantosPostre(u8),
    SelectBucket(Option<usize>),
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    OneHand = 0,
    TwoHands = 1,
}

pub struct ActionPath {
    //pub one_hand_list: Vec<Mano>,
    pub strategy: Strategy<LanceGame>,
    pub buckets: HashMap<AbstractJugada, Vec<Mano>>,
    pub jugadas: Vec<AbstractJugada>,

    pub selected_tantos_mano: Option<u8>,
    pub tantos_mano: Vec<u8>,
    pub selected_tantos_postre: Option<u8>,
    pub tantos_postre: Vec<u8>,
    pub selected_strategy: Option<HandConfiguration>,
    pub strategies: Vec<HandConfiguration>,
    pub selected_actions: Vec<Option<OptionalAction>>,
    pub actions: Vec<Vec<OptionalAction>>,
    pub view_mode: ViewMode,
    pub one_hand_squares: Vec<SquareData<ExplorerEvent>>,
    pub two_hands_squares: Vec<Vec<SquareData<ExplorerEvent>>>,
    pub hovered_square: Option<usize>,
}

impl ActionPath {
    pub fn new(strategy: Strategy<LanceGame>) -> Self {
        let one_hand_list = ActionPath::one_hand_list(&strategy);
        let mut buckets = HashMap::new();
        for hand in &one_hand_list {
            if let Some(lance) = strategy.strategy_config.game_config.lance {
                let jugada = match lance {
                    Lance::Grande => AbstractGrande::abstract_hand(hand),
                    Lance::Chica => AbstractChica::abstract_hand(hand),
                    Lance::Pares => AbstractPares::abstract_hand(hand).unwrap(),
                    Lance::Juego => AbstractJuego::abstract_hand(hand).unwrap(),
                    Lance::Punto => AbstractPunto::abstract_hand(hand),
                };
                let entry = buckets.entry(jugada).or_insert(vec![]);
                entry.push(hand.to_owned());
            }
        }
        let mut jugadas: Vec<_> = buckets.keys().cloned().collect();
        jugadas.sort();
        let mut one_hand_squares = Vec::with_capacity(jugadas.len());
        let mut two_hands_squares = Vec::with_capacity(jugadas.len());
        for (i, jugada) in jugadas.iter().enumerate() {
            one_hand_squares.push(
                SquareData::new(jugada.to_string())
                    .on_hover(move || ExplorerEvent::SelectBucket(Some(i))),
            );
            let mut row = Vec::with_capacity(one_hand_list.len());
            for jugada2 in &jugadas {
                row.push(SquareData::new(format!("{},{}", jugada, jugada2)))
            }
            two_hands_squares.push(row);
        }
        let strategies = match strategy.strategy_config.game_config.lance {
            Some(lance) => match lance {
                Lance::Grande | Lance::Chica | Lance::Punto => vec![HandConfiguration::CuatroManos],
                _ => vec![
                    HandConfiguration::DosManos,
                    HandConfiguration::TresManos1vs2,
                    HandConfiguration::TresManos1vs2Intermedio,
                    HandConfiguration::TresManos2vs1,
                    HandConfiguration::CuatroManos,
                ],
            },
            None => todo!(),
        };
        let mut action_path = Self {
            one_hand_squares,
            two_hands_squares,
            buckets,
            jugadas,
            view_mode: ViewMode::OneHand,
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
        };
        let mano = &action_path.buckets.values().next().unwrap()[0];
        let strategy_node = action_path.strategy_node(mano, None);
        action_path.append_action_picklists(&strategy_node.unwrap().0);
        action_path.update_squares();
        action_path
    }

    fn one_hand_list(s: &Strategy<LanceGame>) -> Vec<Mano> {
        let manos = CartaIter::new(&Carta::CARTAS_MUS, 4).map(Mano::new);
        if let Some(lance) = &s.strategy_config.game_config.lance {
            match lance {
                musolver::mus::Lance::Pares => manos
                    .filter(|m| m.pares().is_some())
                    .sorted_by(|a, b| lance.compara_manos(a, b))
                    .collect(),
                musolver::mus::Lance::Punto => manos
                    .filter(|m| m.valor_puntos() <= 30)
                    .sorted_by(|a, b| lance.compara_manos(a, b))
                    .collect(),
                musolver::mus::Lance::Juego => manos
                    .filter(|m| m.juego().is_some())
                    .sorted_by(|a, b| lance.compara_manos(a, b))
                    .collect(),
                _ => manos.sorted_by(|a, b| lance.compara_manos(a, b)).collect(),
            }
        } else {
            manos.collect()
        }
    }

    fn append_action_picklists(&mut self, actions: &[Accion]) {
        let mut valores: Vec<OptionalAction> = vec![OptionalAction(None)];
        valores.extend(actions.iter().map(|c| OptionalAction(Some(*c))));
        self.selected_actions.push(None);
        self.actions.push(valores);
    }

    fn strategy_node(&self, mano1: &Mano, mano2: Option<&Mano>) -> Option<(Vec<Accion>, Vec<f64>)> {
        let tipo_estrategia = self.selected_strategy.unwrap();
        let history: Vec<Accion> = self.selected_history();
        let lance = self.strategy.strategy_config.game_config.lance;
        let abstract_game = self.strategy.strategy_config.game_config.abstract_game;
        let tantos = [
            self.selected_tantos_mano.unwrap_or_default(),
            self.selected_tantos_postre.unwrap_or_default(),
        ];
        let abstract_game_lance = if abstract_game { lance } else { None };
        let mut lance_game = LanceGame::new(lance.unwrap(), tantos, abstract_game);
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
        self.strategy
            .nodes
            .get(&(info_set + &lance_game.history_str()))
            .cloned()
    }

    fn update_squares(&mut self) {
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
        if self.view_mode == ViewMode::OneHand {
            let action_probability: Vec<Option<_>> = self
                .jugadas
                .iter()
                .map(|jugada| {
                    let manos = self.buckets.get(jugada).unwrap();
                    let probabilities: Vec<(Vec<Accion>, Vec<f64>)> = manos
                        .iter()
                        .filter_map(|hand| self.strategy_node(hand, None))
                        .collect();

                    avg_probability(probabilities)
                })
                .collect();
            action_probability
                .into_iter()
                .zip(&mut self.one_hand_squares)
                .for_each(|(square_data, square)| {
                    match square_data {
                        Some((actions, avg_probability)) => {
                            square.update_with_node(&actions, &avg_probability);
                            //square.mano = jugada.to_string();
                        }
                        None => square.reset_probabilities(),
                    }
                });
        } else {
            for column in 0..self.jugadas.len() {
                for row in 0..self.jugadas.len() {
                    let jugada1 = &self.jugadas[row];
                    let jugada2 = &self.jugadas[column];
                    let (manos1, manos2) = self
                        .buckets
                        .get(jugada1)
                        .zip(self.buckets.get(jugada2))
                        .unwrap();
                    let probabilities: Vec<(Vec<Accion>, Vec<f64>)> = manos1
                        .iter()
                        .cartesian_product(manos2.iter())
                        .filter_map(|(hand1, hand2)| self.strategy_node(hand1, Some(hand2)))
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
    }

    fn selected_history(&self) -> Vec<Accion> {
        self.selected_actions
            .iter()
            .filter_map(|optional_action| optional_action.as_ref())
            .map(|optional_action| optional_action.0.unwrap())
            .collect()
    }

    pub fn update(&mut self, message: ExplorerEvent) {
        match message {
            ExplorerEvent::SetAction(level, action) => {
                self.selected_actions[level] = action.0.map(|_| action);
                self.selected_actions.drain(level + 1..);
                self.actions.drain(level + 1..);

                if action.0.is_some() {
                    let mano = &self.buckets.values().next().unwrap()[0];
                    let strategy_node = self.strategy_node(mano, None);
                    self.append_action_picklists(&strategy_node.unwrap().0);
                }
            }
            ExplorerEvent::SetStrategy(strategy) => {
                self.selected_strategy = Some(strategy);
            }
            ExplorerEvent::SetTantosMano(tantos) => self.selected_tantos_mano = Some(tantos),
            ExplorerEvent::SetTantosPostre(tantos) => self.selected_tantos_postre = Some(tantos),
            ExplorerEvent::SelectBucket(bucket_id) => self.hovered_square = bucket_id,
        }
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
        self.update_squares();
    }

    pub fn view(&self) -> Element<ExplorerEvent> {
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

        let pick_action1 = pick_list(&self.actions[0][..], self.selected_actions[0], |elem| {
            ExplorerEvent::SetAction(0, elem)
        })
        .placeholder("Select an action");
        top_row = top_row.push(pick_action1);

        for level in 1..self.selected_actions.len() {
            let pick_action_n = pick_list(
                &self.actions[level][..],
                self.selected_actions[level],
                move |elem| ExplorerEvent::SetAction(level, elem),
            )
            .placeholder("Select an action");
            top_row = top_row.push(pick_action_n);
        }
        top_row = top_row.width(Fill).align_y(Top).spacing(10);

        let legend =
            Container::new(Canvas::new(Legend::default()).width(700).height(60)).padding(20);

        let bucket_info = self
            .hovered_square
            .map_or_else(
                || column![text("-")],
                |bucket_id| {
                    column![
                        text!("{}", self.one_hand_squares[bucket_id].mano)
                            .center()
                            .size(20),
                        text!("Paso: {:.1}%", self.one_hand_squares[bucket_id].paso * 100.),
                        text!(
                            "Quiero: {:.1}%",
                            self.one_hand_squares[bucket_id].quiero * 100.
                        ),
                        text!(
                            "Envido 2: {:.1}%",
                            self.one_hand_squares[bucket_id].envido2 * 100.
                        ),
                        text!(
                            "Envido 5: {:.1}%",
                            self.one_hand_squares[bucket_id].envido5 * 100.
                        ),
                        text!(
                            "Envido 10: {:.1}%",
                            self.one_hand_squares[bucket_id].envido10 * 100.
                        ),
                        text!(
                            "Órdago: {:.1}%",
                            self.one_hand_squares[bucket_id].ordago * 100.
                        ),
                    ]
                },
            )
            .width(300)
            .padding(20);

        let mut matrix = Column::new();
        if self.view_mode == ViewMode::OneHand {
            for square in &self.one_hand_squares {
                matrix = matrix.push(row![Canvas::new(square).width(50).height(50)])
            }
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
}
