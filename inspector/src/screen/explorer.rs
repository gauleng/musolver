use std::{collections::HashMap, fmt::Display, iter::zip};

use iced::{
    alignment::{
        Horizontal,
        Vertical::{self, Top},
    },
    mouse,
    widget::{
        canvas::{self, Stroke, Text},
        column, pick_list, row, scrollable, Canvas, Column, Container, Row,
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
        HandConfiguration, InfoSet, Strategy,
    },
    ActionNode,
};

#[derive(Clone, Debug)]
pub enum ExplorerEvent {
    SetAction(usize, OptionalAction),
    SetStrategy(HandConfiguration),
    SetTantosMano(u8),
    SetTantosPostre(u8),
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

#[derive(Debug, Default)]
pub struct SquareData {
    pub paso: f64,
    pub quiero: f64,
    pub envido2: f64,
    pub envido5: f64,
    pub envido10: f64,
    pub ordago: f64,
    pub resto: f64,
    pub mano: String,
    pub cache: canvas::Cache,
}

impl SquareData {
    pub fn update_with_node(&mut self, action_node: &Option<Vec<Accion>>, probabilities: &[f64]) {
        if let Some(children) = &action_node {
            self.reset_probabilities();
            children
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
    }

    pub fn reset_probabilities(&mut self) {
        *self = Self {
            mano: self.mano.clone(),
            ..Default::default()
        };
    }
}

impl<AppEvent> canvas::Program<AppEvent> for SquareData {
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    OneHand = 0,
    TwoHands = 1,
}

#[derive(Debug)]
pub struct ActionPath {
    pub one_hand_list: Vec<Mano>,
    pub strategy: Strategy,
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
    pub one_hand_squares: Vec<SquareData>,
    pub two_hands_squares: Vec<Vec<SquareData>>,
}

impl ActionPath {
    pub fn new(strategy: Strategy) -> Self {
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
        for s in &jugadas {
            one_hand_squares.push(SquareData {
                mano: s.to_string(),
                ..SquareData::default()
            });
            let mut row = Vec::with_capacity(one_hand_list.len());
            for s2 in &jugadas {
                row.push(SquareData {
                    mano: format!("{},{}", s, s2),
                    ..SquareData::default()
                })
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
            view_mode: ViewMode::TwoHands,
            one_hand_list,
            strategy: strategy.to_owned(),
            selected_tantos_mano: Some(0),
            tantos_mano: Vec::from_iter(0..40),
            selected_tantos_postre: Some(0),
            tantos_postre: Vec::from_iter(0..40),
            selected_actions: vec![],
            actions: vec![],
            selected_strategy: Some(HandConfiguration::CuatroManos),
            strategies,
        };
        action_path.append_action_picklists(&strategy.strategy_config.trainer_config.action_tree);
        action_path.update_squares();
        action_path
    }

    fn one_hand_list(s: &Strategy) -> Vec<Mano> {
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

    fn append_action_picklists(&mut self, action_node: &ActionNode<usize, Accion>) {
        if let ActionNode::NonTerminal(_, children) = action_node {
            let mut valores: Vec<OptionalAction> = vec![OptionalAction(None)];
            valores.extend(
                children
                    .iter()
                    .filter(|c| c.1 != ActionNode::Terminal)
                    .map(|c| OptionalAction(Some(c.0))),
            );
            self.selected_actions.push(None);
            self.actions.push(valores);
        }
    }

    fn update_squares(&mut self) {
        let tipo_estrategia = self.selected_strategy.unwrap();
        let history: Vec<Accion> = self
            .selected_actions
            .iter()
            .filter_map(|a| if let Some(action) = a { action.0 } else { None })
            .collect();
        let lance = self.strategy.strategy_config.game_config.lance;
        let abstract_game = self.strategy.strategy_config.game_config.abstract_game;
        let actions = self.selected_action_node().actions();
        let tantos = [
            self.selected_tantos_mano.unwrap_or_default(),
            self.selected_tantos_postre.unwrap_or_default(),
        ];
        let abstract_game = if abstract_game { lance } else { None };

        if self.view_mode == ViewMode::OneHand {
            for (jugada, square) in zip(&self.jugadas, &mut self.one_hand_squares) {
                if let Some(manos) = self.buckets.get(jugada) {
                    let probabilities: Vec<_> = manos
                        .iter()
                        .filter_map(|mano| {
                            let info_set = InfoSet::str(
                                &tipo_estrategia,
                                &tantos,
                                mano,
                                None,
                                &history,
                                abstract_game,
                            );
                            self.strategy.nodes.get(&info_set).cloned()
                        })
                        .collect();
                    let n = probabilities.len();
                    let avg_probability = probabilities
                        .into_iter()
                        .reduce(|avg, v| zip(avg, v).map(|(a, v)| a + v / n as f64).collect())
                        .unwrap();
                    square.update_with_node(&actions, &avg_probability);
                    square.mano = jugada.to_string();
                }
            }
        } else {
            for column in 0..self.jugadas.len() {
                for row in 0..self.jugadas.len() {
                    let square = &mut self.two_hands_squares[row][column];
                    let jugada1 = &self.jugadas[row];
                    let jugada2 = &self.jugadas[column];
                    let manos = self.buckets.get(jugada1).zip(self.buckets.get(jugada2));
                    if let Some((manos1, manos2)) = manos {
                        let probabilities: Vec<_> = manos1
                            .iter()
                            .cartesian_product(manos2.iter())
                            .filter_map(|(hand1, hand2)| {
                                let info_set = InfoSet::str(
                                    &tipo_estrategia,
                                    &tantos,
                                    hand1,
                                    Some(hand2),
                                    &history,
                                    abstract_game,
                                );
                                self.strategy.nodes.get(&info_set).cloned()
                            })
                            .collect();
                        let n = probabilities.len();
                        let avg_probability = probabilities
                            .into_iter()
                            .reduce(|avg, v| zip(avg, v).map(|(a, v)| a + v / n as f64).collect());
                        if let Some(probabilities) = avg_probability {
                            square.update_with_node(&actions, &probabilities);
                            square.mano = format!("{jugada1},{jugada2}");
                        }
                    }
                }
            }
        }
    }

    fn selected_action_node(&self) -> &ActionNode<usize, Accion> {
        let mut current_node = &self.strategy.strategy_config.trainer_config.action_tree;
        for s in &self.selected_actions {
            if let Some(a) = s {
                current_node = current_node.next_node(a.0.unwrap()).unwrap();
            } else {
                break;
            }
        }
        current_node
    }

    pub fn update(&mut self, message: ExplorerEvent) {
        match message {
            ExplorerEvent::SetAction(level, action) => {
                self.selected_actions[level] = action.0.map(|_| action);
                self.selected_actions.drain(level + 1..);
                self.actions.drain(level + 1..);

                if action.0.is_some() {
                    let current_node = self.selected_action_node();
                    self.append_action_picklists(&current_node.to_owned());
                }
            }
            ExplorerEvent::SetStrategy(strategy) => {
                let turn = self.selected_action_node().to_play();
                self.view_mode = match strategy {
                    HandConfiguration::DosManos => ViewMode::OneHand,
                    HandConfiguration::CuatroManos => ViewMode::TwoHands,
                    HandConfiguration::TresManos1vs2
                    | HandConfiguration::TresManos1vs2Intermedio => {
                        if turn.unwrap() == 0 {
                            ViewMode::OneHand
                        } else {
                            ViewMode::TwoHands
                        }
                    }
                    HandConfiguration::TresManos2vs1 => {
                        if turn.unwrap() == 0 {
                            ViewMode::TwoHands
                        } else {
                            ViewMode::OneHand
                        }
                    }
                    HandConfiguration::SinLance => ViewMode::OneHand,
                };
                self.selected_strategy = Some(strategy);
            }
            ExplorerEvent::SetTantosMano(tantos) => self.selected_tantos_mano = Some(tantos),
            ExplorerEvent::SetTantosPostre(tantos) => self.selected_tantos_postre = Some(tantos),
        }
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

        let pick_tantos_mano =
            pick_list(&self.tantos_mano[..], Some(0), ExplorerEvent::SetTantosMano);
        top_row = top_row.push(pick_tantos_mano);

        let pick_tantos_postre = pick_list(
            &self.tantos_postre[..],
            Some(0),
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

        let scrollable_matrix = scrollable(matrix)
            .direction(scrollable::Direction::Both {
                vertical: scrollable::Scrollbar::default(),
                horizontal: scrollable::Scrollbar::default(),
            })
            .width(Length::Fill);
        let layout = column![top_row, legend, scrollable_matrix].align_x(Horizontal::Center);

        layout.into()
    }
}
