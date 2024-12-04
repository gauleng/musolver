use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use iced::{
    futures::{channel::mpsc, SinkExt, Stream, StreamExt},
    widget::{button, column, container, row, scrollable, text},
    Alignment, Element, Length, Task,
};
use image::GenericImageView;
use musolver::{
    mus::{
        arena::{ActionRecorder, Agent, AgenteMusolver, Kibitzer, MusAction, MusArena},
        Accion, Lance, Mano,
    },
    solver::{LanceGame, Strategy},
    Game,
};

#[derive(Debug, Clone)]
pub struct Connection {
    to_agent: mpsc::Sender<ArenaCommand>,
    to_arena: mpsc::Sender<ArenaCommand>,
}
impl Connection {
    fn pick_action(&mut self, accion: Accion) {
        let _ = self.to_agent.try_send(ArenaCommand::PickAction(accion));
    }

    fn new_game(&mut self) {
        let _ = self.to_arena.try_send(ArenaCommand::NewGame);
    }

    fn terminate(&mut self) {
        let _ = self.to_arena.try_send(ArenaCommand::Terminate);
    }
}

enum ArenaState {
    Disconnected,
    Connected(Connection),
}

#[derive(Debug, Clone)]
pub enum ArenaMessage {
    AgentInitialized(Connection),
    GameAction(MusAction),
    ActionRequested(Vec<Accion>),
    NewGameRequested,
}

enum ArenaCommand {
    PickAction(Accion),
    NewGame,
    Terminate,
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    NewGame,
    Close,
    ArenaMessage(ArenaMessage),
    ActionSelected(Accion),
}

pub enum GameAction {
    OpenLoader,
}

#[derive(Debug, Clone, Default)]
pub struct Player {
    name: String,
    hand: Mano,
    last_action: Option<Accion>,
}

pub struct MusArenaUi {
    state: ArenaState,
    arena_events: Vec<MusAction>,
    deck_images: DeckImages,
    actions: Vec<Accion>,
    players: [Player; 4],
    dealer: usize,
    scoreboard: [u8; 2],
    game_running: bool,
    lance: Option<Lance>,
}

impl MusArenaUi {
    pub fn new(strategy: Strategy<LanceGame>) -> (Self, Task<GameEvent>) {
        let task = Task::run(setup_arena(strategy), GameEvent::ArenaMessage);
        (
            Self {
                state: ArenaState::Disconnected,
                actions: vec![],
                players: [
                    Player {
                        name: "Hero".to_string(),
                        ..Default::default()
                    },
                    Player {
                        name: "Musolver".to_string(),
                        ..Default::default()
                    },
                    Player {
                        name: "Musolver".to_string(),
                        ..Default::default()
                    },
                    Player {
                        name: "Musolver".to_string(),
                        ..Default::default()
                    },
                ],
                dealer: 0,
                arena_events: vec![],
                scoreboard: [0, 0],
                deck_images: deck(),
                game_running: false,
                lance: None,
            },
            task,
        )
    }

    pub fn view(&self) -> Element<GameEvent> {
        let scoreboard = container(row![text(format!(
            "{} - {}",
            self.scoreboard[0], self.scoreboard[1]
        ))
        .size(40)])
        .style(container::rounded_box)
        .padding(10);

        let hands = container(
            column![
                self.hand_row(2, 60),
                row![
                    self.hand_row(3, 60),
                    container(text("Pot: "))
                        .width(Length::Fill)
                        .align_x(Alignment::Center),
                    self.hand_row(1, 60),
                ]
                .align_y(Alignment::Center),
                self.hand_row(0, 100),
            ]
            .spacing(10)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill);

        let history = container(
            scrollable(column(self.arena_events.iter().map(|mus_action| {
                match mus_action {
                    MusAction::LanceStart(lance) => text(format!("Lance start: {lance:?}")),
                    MusAction::PlayerAction(player_id, accion) => {
                        text(format!("Player {player_id}: {accion}"))
                    }
                    MusAction::Payoff(pareja_id, tantos) => {
                        text(format!("Payoff: Couple {pareja_id} wins {tantos} tantos"))
                    }
                    _ => text(""),
                }
                .into()
            })))
            .anchor_bottom()
            .width(Length::Fill),
        )
        .style(container::bordered_box)
        .width(300);

        let actions = if self.game_running {
            row(self.actions.iter().map(|action| {
                button(
                    text(action.to_string())
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center),
                )
                .width(120)
                .height(40)
                .on_press(GameEvent::ActionSelected(*action))
                .into()
            }))
        } else {
            row![
                button(
                    text("New game")
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center),
                )
                .width(120)
                .height(40)
                .on_press(GameEvent::NewGame),
                button(
                    text("Close arena")
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center),
                )
                .width(120)
                .height(40)
                .on_press(GameEvent::Close)
            ]
        }
        .spacing(10)
        .padding(10);

        column![
            scoreboard,
            row![hands, history.height(Length::Fill)]
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .padding(10)
                .spacing(10),
            actions
        ]
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .into()
    }

    fn hand_row(
        &self,
        player_id: usize,
        card_width: u16,
    ) -> iced::widget::Container<'_, GameEvent> {
        let hand = &self.players[player_id].hand;
        let is_dealer = player_id == self.dealer;
        let visible = player_id == 0 || !self.game_running;
        let active = if let Some(lance) = self.lance {
            hand.jugada(&lance).is_some()
        } else {
            false
        };

        let cards_row = row![
            if visible {
                row(hand.cartas().iter().map(|carta| {
                    iced::widget::image(
                        self.deck_images.cards[carta.valor() as usize - 1][0].clone(),
                    )
                    .width(card_width)
                    .into()
                }))
            } else {
                row![
                    iced::widget::image(self.deck_images.back.clone()).width(card_width),
                    iced::widget::image(self.deck_images.back.clone()).width(card_width),
                    iced::widget::image(self.deck_images.back.clone()).width(card_width),
                    iced::widget::image(self.deck_images.back.clone()).width(card_width),
                ]
            },
            text(if is_dealer { "(M)" } else { "" })
        ]
        .align_y(Alignment::Center);

        container(column![
            text(&self.players[player_id].name),
            cards_row,
            text(
                self.players[player_id]
                    .last_action
                    .map_or_else(|| "".to_string(), |action| action.to_string())
            )
        ])
        .padding(5)
        .style(if active {
            container::rounded_box
        } else {
            container::dark
        })
    }

    pub fn update(&mut self, message: GameEvent) -> Option<GameAction> {
        match message {
            GameEvent::ArenaMessage(mus_action) => match mus_action {
                ArenaMessage::AgentInitialized(connection) => {
                    println!("Agent initialized...");
                    self.state = ArenaState::Connected(connection);
                    None
                }
                ArenaMessage::GameAction(mus_action) => {
                    match mus_action {
                        MusAction::GameStart(dealer_id) => {
                            self.dealer = dealer_id;
                            self.game_running = true;
                            self.players.iter_mut().for_each(|player| {
                                player.last_action = None;
                            });
                        }
                        MusAction::DealHand(player_id, mano) => {
                            self.players[player_id].hand = mano.clone();
                        }
                        MusAction::Payoff(couple_id, tantos) => {
                            self.scoreboard[couple_id] += tantos;
                            self.arena_events.push(mus_action);
                        }
                        MusAction::LanceStart(lance) => {
                            self.lance = Some(lance);
                            self.arena_events.push(mus_action);
                        }
                        MusAction::PlayerAction(player_id, action) => {
                            self.players[player_id].last_action = Some(action);
                            self.arena_events.push(mus_action);
                        }
                    }
                    None
                }
                ArenaMessage::ActionRequested(actions) => {
                    self.actions = actions;
                    None
                }
                ArenaMessage::NewGameRequested => {
                    if let ArenaState::Connected(_connection) = &mut self.state {
                        self.game_running = false;
                    }
                    None
                }
            },
            GameEvent::ActionSelected(accion) => {
                if let ArenaState::Connected(connection) = &mut self.state {
                    connection.pick_action(accion);
                }
                None
            }
            GameEvent::NewGame => {
                if let ArenaState::Connected(connection) = &mut self.state {
                    connection.new_game();
                }
                None
            }
            GameEvent::Close => {
                if let ArenaState::Connected(connection) = &mut self.state {
                    connection.terminate();
                }
                Some(GameAction::OpenLoader)
            }
        }
    }
}

fn setup_arena(strategy: Strategy<LanceGame>) -> impl Stream<Item = ArenaMessage> {
    iced::stream::channel(100, move |mut sender| async move {
        struct KibitzerGui {
            sender: mpsc::Sender<ArenaMessage>,
        }
        impl KibitzerGui {
            fn new(sender: mpsc::Sender<ArenaMessage>) -> Self {
                Self { sender }
            }
        }
        impl Kibitzer for KibitzerGui {
            fn record(&mut self, _partida_mus: &musolver::mus::PartidaMus, action: MusAction) {
                let _ = self.sender.try_send(ArenaMessage::GameAction(action));
            }
        }

        struct AgentGui {
            sender: mpsc::Sender<ArenaMessage>,
            receiver: mpsc::Receiver<ArenaCommand>,
            history: Arc<Mutex<Vec<Accion>>>,
        }
        impl AgentGui {
            fn new(
                sender: mpsc::Sender<ArenaMessage>,
                receiver: mpsc::Receiver<ArenaCommand>,
                history: Arc<Mutex<Vec<Accion>>>,
            ) -> Self {
                Self {
                    sender,
                    receiver,
                    history,
                }
            }
        }
        #[async_trait]
        impl Agent for AgentGui {
            async fn actuar(
                &mut self,
                partida_mus: &musolver::mus::PartidaMus,
            ) -> musolver::mus::Accion {
                let mut lance_game = LanceGame::from_partida_mus(partida_mus, true).unwrap();
                for action in self.history.lock().unwrap().iter() {
                    lance_game.act(*action);
                }
                let next_actions = lance_game.actions();
                let _ = self
                    .sender
                    .try_send(ArenaMessage::ActionRequested(next_actions));
                if let ArenaCommand::PickAction(action) = self.receiver.next().await.unwrap() {
                    action
                } else {
                    Accion::Paso
                }
            }
        }
        let (to_agent, receiver_agent) = mpsc::channel(100);
        let (to_arena, mut receiver_arena) = mpsc::channel(100);
        let _ = sender.try_send(ArenaMessage::AgentInitialized(Connection {
            to_agent,
            to_arena,
        }));
        let lance = strategy.strategy_config.game_config.lance;
        let mut arena = MusArena::new(lance);
        let kibitzer = KibitzerGui::new(sender.clone());
        let action_recorder = ActionRecorder::new();
        let agent_musolver = AgenteMusolver::new(strategy, action_recorder.history());
        let agent_gui = AgentGui::new(sender.clone(), receiver_agent, action_recorder.history());

        arena.agents.push(Box::new(agent_gui));
        arena.agents.push(Box::new(agent_musolver.clone()));
        arena.agents.push(Box::new(agent_musolver.clone()));
        arena.agents.push(Box::new(agent_musolver.clone()));
        arena.kibitzers.push(Box::new(kibitzer));
        arena.kibitzers.push(Box::new(action_recorder));
        loop {
            arena.start().await;
            let _ = sender.send(ArenaMessage::NewGameRequested).await;
            if let ArenaCommand::Terminate = receiver_arena.next().await.unwrap() {
                break;
            }
        }
    })
}

struct DeckImages {
    cards: Vec<Vec<iced::widget::image::Handle>>,
    back: iced::widget::image::Handle,
}

fn deck() -> DeckImages {
    let image = image::open("inspector/assets/Baraja_española_completa.png")
        .expect("Deck image file should be in assets folder.");
    let dim = image.dimensions();
    let (rows, cols) = (5, 12);
    let (card_width, card_height) = (dim.0 / cols, dim.1 / rows);
    let mut cards = vec![vec![]; cols as usize];
    for row in 0..rows - 1 {
        for col in 0..cols {
            let x = col * card_width;
            let y = row * card_height;
            let card = image.crop_imm(x, y, card_width, card_height);
            let buffer = card.to_rgba8().into_raw();
            cards[col as usize].push(iced::widget::image::Handle::from_rgba(
                card_width,
                card_height,
                buffer,
            ));
        }
    }
    let buffer_back = image
        .crop_imm(card_width, 4 * card_height, card_width, card_height)
        .to_rgba8()
        .into_raw();

    let back = iced::widget::image::Handle::from_rgba(card_width, card_height, buffer_back);
    DeckImages { cards, back }
}
