use std::{collections::HashMap, fmt::Display, iter::zip, sync::Arc};

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
        checkbox, column, pick_list, row, scrollable, text,
    },
};
use itertools::Itertools;
use musolver::{
    mus::{Accion, FasePartida, Lance, RankingManos},
    solver::{
        AbstractJugada, Cursor, CursorMove, CursorNode, GameType, HandConfig, HandConfiguration,
        HandKind, SolverError, StrategyReader,
    },
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    OneHand = 0,
    TwoHands = 1,
}

pub struct ActionPath {
    pub cursor: Cursor,
    pub selected_tantos_mano: Option<u8>,
    pub tantos_mano: Vec<u8>,
    pub selected_tantos_postre: Option<u8>,
    pub tantos_postre: Vec<u8>,
    pub selected_strategy: Option<HandConfiguration>,
    pub strategies: Vec<HandConfiguration>,
    pub selected_actions: Vec<Option<OptionalAction>>,
    pub actions: Vec<(FasePartida, u8, Vec<OptionalAction>)>,
    pub view_mode: ViewMode,
    pub one_hand_squares: Vec<(AbstractJugada, SquareData<ExplorerEvent>)>,
    pub two_hands_squares: Vec<Vec<SquareData<ExplorerEvent>>>,
    pub hovered_square: Option<usize>,
    pub jugadas: Vec<HandConfig>,
    /// Último error del cursor: los eventos de la interfaz no pueden propagarlo, así que se
    /// guarda para mostrarlo.
    pub error: Option<String>,
}

impl ActionPath {
    pub fn new(strategy: Arc<StrategyReader>) -> Self {
        let game_type = strategy.strategy_config().game_config.game_type;
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
        let cursor = strategy.cursor();

        let mut action_path = Self {
            one_hand_squares: vec![],
            two_hands_squares: vec![],
            view_mode: match game_type {
                GameType::MusGameTwoHands => ViewMode::TwoHands,
                _ => ViewMode::OneHand,
            },
            cursor,
            selected_tantos_mano: Some(0),
            tantos_mano: Vec::from_iter(0..40),
            selected_tantos_postre: Some(0),
            tantos_postre: Vec::from_iter(0..40),
            selected_actions: vec![],
            actions: vec![],
            selected_strategy: Some(HandConfiguration::CuatroManos),
            strategies,
            hovered_square: None,
            error: None,
            jugadas: vec![
                HandConfig {
                    pares: true,
                    juego: true,
                };
                game_type.num_hands()
            ],
        };
        action_path.update_squares();
        action_path
    }

    pub fn update(&mut self, message: ExplorerEvent) {
        self.hovered_square = None;
        match message {
            ExplorerEvent::SetAction(level, action) => {
                // El nivel del desplegable es la posición en el recorrido del cursor: se retrocede
                // hasta él y se actúa, lo que descarta por su cuenta lo que hubiera después.
                self.cursor.seek(level);
                match action.0 {
                    Some(accion) => {
                        // Puede fallar si el desplegable se quedó obsoleto respecto al cursor.
                        let resultado = self.cursor.act(CursorMove::Play(accion));
                        self.report(resultado);
                    }
                    // La opción vacía corta el recorrido en ese desplegable.
                    None => self.cursor.truncate(),
                }
            }
            ExplorerEvent::SetStrategy(strategy) => {
                self.selected_strategy = Some(strategy);
            }
            ExplorerEvent::SetTantosMano(tantos) => {
                self.selected_tantos_mano = Some(tantos);
                self.cursor.set_tantos([
                    self.selected_tantos_mano.unwrap(),
                    self.selected_tantos_postre.unwrap(),
                ]);
            }
            ExplorerEvent::SetTantosPostre(tantos) => {
                self.selected_tantos_postre = Some(tantos);
                self.cursor.set_tantos([
                    self.selected_tantos_mano.unwrap(),
                    self.selected_tantos_postre.unwrap(),
                ]);
            }
            ExplorerEvent::SelectBucket(bucket_id) => {
                self.hovered_square = bucket_id;
                return;
            }
            ExplorerEvent::SetPares(player, jugada) => {
                self.jugadas[player].pares = jugada;
                let resultado = self.cursor.set_hand_config(player, self.jugadas[player]);
                self.report(resultado);
            }
            ExplorerEvent::SetJuego(player, jugada) => {
                self.jugadas[player].juego = jugada;
                let resultado = self.cursor.set_hand_config(player, self.jugadas[player]);
                self.report(resultado);
            }
        }
        // Los desplegables se rehacen desde el cursor, así que no hay que recortarlos a mano.
        self.update_squares();
    }

    /// Guarda el resultado de una operación del cursor para enseñarlo: si sale bien, borra el
    /// mensaje anterior.
    fn report(&mut self, resultado: Result<(), SolverError>) {
        self.error = resultado.err().map(|err| err.to_string());
    }

    fn update_squares(&mut self) {
        let lance = match self.cursor.phase() {
            Some(FasePartida::Envites(lance)) => lance,
            _ => Lance::Grande,
        };
        match self.view_mode {
            ViewMode::OneHand => self.update_squares_one_hand(&lance),
            ViewMode::TwoHands => self.update_squares_two_hands(&lance),
        }
        self.rebuild_action_picklists();
    }

    /// Rehace los desplegables a partir del cursor: uno por nodo del recorrido, con la accion
    /// elegida en cada uno.
    ///
    /// Antes se acumulaban desplegable a desplegable, lo que se desincroniza en cuanto el
    /// cursor recorta la linea por su cuenta al cambiar los tantos o una jugada. El nivel de cada
    /// desplegable es la posicion en el cursor, asi que no se salta ninguna.
    fn rebuild_action_picklists(&mut self) {
        self.actions.clear();
        self.selected_actions.clear();
        for level in 0..self.cursor.history_len() {
            let (Some(phase), Some(turno)) =
                (self.cursor.phase_at(level), self.cursor.turn_at(level))
            else {
                continue;
            };
            // TODO: en la fase de descartes hay que ofrecer `DiscardAction`, no acciones sueltas.
            let opciones = match self.cursor.node_at(level) {
                CursorNode::Play(actions) => actions,
                CursorNode::Discard | CursorNode::Terminal => vec![],
            };
            let mut valores: Vec<OptionalAction> = vec![OptionalAction(None)];
            valores.extend(opciones.iter().map(|accion| OptionalAction(Some(*accion))));
            self.actions.push((phase, turno.player_id(), valores));

            // Solo los movimientos que llegan a jugarse cuentan como elegidos: la cola que el
            // cursor conserva sin realizar se muestra vacia.
            let seleccionada = (level + 1 < self.cursor.history_len())
                .then(|| self.cursor.moves().get(level))
                .flatten()
                .and_then(|movimiento| match movimiento {
                    CursorMove::Play(accion) => Some(OptionalAction(Some(*accion))),
                    CursorMove::Discard(_) => None,
                });
            self.selected_actions.push(seleccionada);
        }
    }

    fn update_squares_two_hands(&mut self, lance: &Lance) {
        let Ok(strategies) = self.cursor.strategies() else {
            self.two_hands_squares.clear();
            return;
        };

        let mut squares: HashMap<(AbstractJugada, AbstractJugada), (Vec<Accion>, Vec<f64>, usize)> =
            HashMap::new();
        for strategy in strategies {
            let HandKind::TwoHands(mano1, mano2) = strategy.hand() else {
                continue;
            };
            let (Some(jugada1), Some(jugada2)) = (
                AbstractJugada::to_abstract(mano1, lance),
                AbstractJugada::to_abstract(mano2, lance),
            ) else {
                continue;
            };
            let celda = squares.entry((jugada1, jugada2)).or_insert_with(|| {
                (
                    strategy.actions().to_vec(),
                    vec![0.; strategy.strategy().len()],
                    0,
                )
            });
            for (acumulado, probabilidad) in zip(&mut celda.1, strategy.strategy()) {
                *acumulado += probabilidad;
            }
            celda.2 += 1;
        }

        let jugadas: Vec<AbstractJugada> = squares
            .keys()
            .flat_map(|(jugada1, jugada2)| [*jugada1, *jugada2])
            .sorted()
            .dedup()
            .collect();

        let mut bucket_id = 0;
        self.two_hands_squares = jugadas
            .iter()
            .map(|jugada1| {
                jugadas
                    .iter()
                    .map(|jugada2| {
                        let mut square = SquareData::new(format!("{jugada1},{jugada2}"))
                            .on_hover(move || ExplorerEvent::SelectBucket(Some(bucket_id)));
                        bucket_id += 1;
                        if let Some((actions, suma, num_pares)) = squares.get(&(*jugada1, *jugada2))
                        {
                            let media: Vec<f64> =
                                suma.iter().map(|v| v / *num_pares as f64).collect();
                            square.update_with_node(actions, &media);
                        }
                        square
                    })
                    .collect()
            })
            .collect();
    }

    fn update_squares_one_hand(&mut self, lance: &Lance) {
        let Ok(strategies) = self.cursor.strategies() else {
            self.one_hand_squares.clear();
            return;
        };
        self.one_hand_squares = strategies
            .into_iter()
            .filter_map(|strategy| match strategy.hand() {
                HandKind::OneHand(mano) => {
                    let jugada = AbstractJugada::to_abstract(mano, lance)?;
                    Some((jugada, mano.clone(), strategy))
                }
                HandKind::TwoHands(_, _) => None,
            })
            .sorted_by(|(jugada1, mano1, _), (jugada2, mano2, _)| {
                jugada1
                    .cmp(jugada2)
                    .then_with(|| lance.compara_manos(mano1, mano2))
            })
            .enumerate()
            .map(|(bucket_id, (jugada, mano, strategy))| {
                let mut square_data = SquareData::new(mano.to_string())
                    .on_hover(move || ExplorerEvent::SelectBucket(Some(bucket_id)));
                square_data.update_with_node(strategy.actions(), strategy.strategy());
                (jugada, square_data)
            })
            .collect();
    }

    pub fn view(&self) -> Element<'_, ExplorerEvent> {
        let top_row = match self.cursor.game_type() {
            GameType::LanceGame(_) | GameType::LanceGameTwoHands(_) => self.nav_bar_lance_game(),
            _ => self.nav_bar_mus_game(),
        };

        let legend =
            Container::new(Canvas::new(Legend::default()).width(700).height(60)).padding(20);

        let jugadas = column(self.jugadas.iter().enumerate().map(|(i, jugada)| {
            column![
                text(format!("Jugador {}", i + 1)),
                row![
                    checkbox("Pares", jugada.pares)
                        .on_toggle(move |v| ExplorerEvent::SetPares(i, v)),
                    checkbox("Juego", jugada.juego)
                        .on_toggle(move |v| ExplorerEvent::SetJuego(i, v))
                ]
            ]
            .into()
        }));

        let bucket_info = self
            .hovered_square
            .map_or_else(
                || column![text("-")],
                |bucket_id| {
                    column(
                        self.one_hand_squares[bucket_id]
                            .1
                            .dist
                            .iter()
                            .map(|(action, probability)| {
                                text(format!(
                                    "{}: {:.1}%",
                                    action_text(action),
                                    probability * 100.
                                ))
                            })
                            .map(Element::from),
                    )
                },
            )
            .width(300)
            .padding(20);

        let sidebar = column![jugadas, bucket_info];

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
            sidebar,
            scrollable(matrix)
                .direction(scrollable::Direction::Both {
                    vertical: scrollable::Scrollbar::default(),
                    horizontal: scrollable::Scrollbar::default(),
                })
                .width(Length::Fill)
        ];
        let error: Element<_> = match &self.error {
            Some(mensaje) => text(mensaje).color(Color::parse("C7253E").unwrap()).into(),
            None => text("").into(),
        };
        let layout = column![top_row, error, legend, scrollable_matrix].align_x(Horizontal::Center);

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
}

#[derive(Clone, Debug)]
pub enum ExplorerEvent {
    SetAction(usize, OptionalAction),
    SetStrategy(HandConfiguration),
    SetTantosMano(u8),
    SetTantosPostre(u8),
    SelectBucket(Option<usize>),
    SetPares(usize, bool),
    SetJuego(usize, bool),
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

fn action_text(action: &Accion) -> String {
    match action {
        Accion::Paso => "Paso".to_string(),
        Accion::Quiero => "Quiero".to_string(),
        Accion::Envido(2) => "Envido 2".to_string(),
        Accion::Envido(5) => "Envido 5".to_string(),
        Accion::Envido(10) => "Envido 10".to_string(),
        Accion::Ordago => "Órdago".to_string(),
        Accion::Mus => "Mus".to_string(),
        Accion::NoMus => "No mus".to_string(),
        Accion::Descartar([c1, c2, c3, c4]) => format!(
            "Descartar {}{}{}{}",
            *c1 as u8, *c2 as u8, *c3 as u8, *c4 as u8
        ),
        _ => "".to_string(),
    }
}

fn action_style(action: &Accion) -> Color {
    match action {
        Accion::Paso => Color::parse("006E90").unwrap(),
        Accion::Quiero => Color::parse("2F9332").unwrap(),
        Accion::Envido(2) => Color::parse("FABC3F").unwrap(),
        Accion::Envido(5) => Color::parse("E85C0D").unwrap(),
        Accion::Envido(10) => Color::parse("C7253E").unwrap(),
        Accion::Ordago => Color::parse("821131").unwrap(),
        Accion::Mus => Color::parse("ADD8E6").unwrap(),
        Accion::NoMus => Color::parse("6495ED").unwrap(),
        Accion::Descartar(_) => Color::parse("800000").unwrap(),
        _ => Color::new(0., 0., 0., 0.),
    }
}

fn draw_order(action: &Accion) -> u8 {
    match action {
        Accion::Paso => 3,
        Accion::Quiero => 4,
        Accion::Envido(2) => 5,
        Accion::Envido(5) => 6,
        Accion::Envido(10) => 7,
        Accion::Ordago => 8,
        Accion::Mus => 0,
        Accion::NoMus => 1,
        Accion::Descartar(_) => 2,
        _ => 100,
    }
}
pub struct SquareData<Message> {
    dist: Vec<(Accion, f64)>,
    pub label: String,
    pub cache: canvas::Cache,
    on_hover: Option<Box<dyn Fn() -> Message + 'static>>,
}

impl<Message> SquareData<Message> {
    pub fn new(label: String) -> Self {
        Self {
            dist: vec![],
            label,
            cache: canvas::Cache::default(),
            on_hover: None,
        }
    }

    pub fn on_hover(mut self, on_hover: impl Fn() -> Message + 'static) -> Self {
        self.on_hover = Some(Box::new(on_hover));
        self
    }

    pub fn update_with_node(&mut self, actions: &[Accion], probabilities: &[f64]) {
        self.reset_probabilities();
        self.cache.clear();
        self.dist = std::iter::zip(actions, probabilities)
            .map(|(a, b)| (*a, *b))
            .sorted_by_key(|(a, _)| draw_order(a))
            .collect();
    }

    pub fn reset_probabilities(&mut self) {
        self.dist.clear();
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
            let region_widths: Vec<f32> = self
                .dist
                .iter()
                .map(|(_, probability)| *probability as f32 * width)
                .collect();
            let region_colors: Vec<Color> = self
                .dist
                .iter()
                .map(|(action, _)| action_style(action))
                .collect();
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
                content: String::from(&self.label),
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
