use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use iced::{
    futures::{channel::mpsc, Stream, StreamExt},
    widget::{button, column, container, row, scrollable, text},
    Alignment, Element, Length, Task,
};
use image::GenericImageView;
use musolver::{
    mus::{
        arena::{ActionRecorder, Agent, AgenteMusolver, Kibitzer, MusAction, MusArena},
        Accion, Mano,
    },
    solver::{LanceGame, Strategy},
    Game,
};

#[derive(Debug, Clone)]
pub enum ArenaMessage {
    AgentInitialized(mpsc::Sender<Accion>),
    GameAction(MusAction),
    ActionRequested(Vec<Accion>),
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    ArenaMessage(ArenaMessage),
    ActionSelected(Accion),
}

pub struct MusArenaUi {
    to_agent: Option<mpsc::Sender<Accion>>,
    arena_events: Vec<MusAction>,
    actions: Vec<Accion>,
    hands: [Mano; 4],
    dealer: usize,
    scoreboard: [u8; 2],
    deck_images: DeckImages,
}

impl MusArenaUi {
    pub fn new(strategy: Strategy<LanceGame>) -> (Self, Task<GameEvent>) {
        let task = Task::run(setup_arena(strategy), GameEvent::ArenaMessage);
        (
            Self {
                to_agent: None,
                actions: vec![],
                hands: [
                    Mano::default(),
                    Mano::default(),
                    Mano::default(),
                    Mano::default(),
                ],
                dealer: 0,
                arena_events: vec![],
                scoreboard: [0, 0],
                deck_images: deck(),
            },
            task,
        )
    }

    pub fn view(&self) -> Element<GameEvent> {
        let hand_row = |hand: &Mano, visible, is_dealer| {
            row![
                if visible {
                    row(hand.cartas().iter().map(|carta| {
                        iced::widget::image(
                            self.deck_images.cards[carta.valor() as usize - 1][0].clone(),
                        )
                        .width(100)
                        .into()
                    }))
                } else {
                    row![
                        iced::widget::image(self.deck_images.back.clone()).width(60),
                        iced::widget::image(self.deck_images.back.clone()).width(60),
                        iced::widget::image(self.deck_images.back.clone()).width(60),
                        iced::widget::image(self.deck_images.back.clone()).width(60),
                    ]
                },
                text(if is_dealer { "(M)" } else { "" })
            ]
            .align_y(Alignment::Center)
        };

        let scoreboard = container(row![text(format!(
            "{} - {}",
            self.scoreboard[0], self.scoreboard[1]
        ))
        .size(40)])
        .style(container::rounded_box)
        .padding(10);

        let hands = container(
            column![
                hand_row(&self.hands[0], true, self.dealer == 0),
                row![
                    hand_row(&self.hands[1], false, self.dealer == 1),
                    container(text("Pot: "))
                        .width(Length::Fill)
                        .align_x(Alignment::Center),
                    hand_row(&self.hands[3], false, self.dealer == 3)
                ]
                .align_y(Alignment::Center)
                .height(100),
                hand_row(&self.hands[2], false, self.dealer == 2),
            ]
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

        let actions = row(self.actions.iter().map(|action| {
            button(
                text(action.to_string())
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center),
            )
            .width(80)
            .height(40)
            .on_press(GameEvent::ActionSelected(*action))
            .into()
        }))
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

    pub fn update(&mut self, message: GameEvent) {
        match message {
            GameEvent::ArenaMessage(mus_action) => match mus_action {
                ArenaMessage::AgentInitialized(sender) => {
                    println!("Agent initialized...");
                    self.to_agent = Some(sender);
                }
                ArenaMessage::GameAction(mus_action) => match mus_action {
                    MusAction::GameStart(dealer_id) => {
                        self.dealer = dealer_id;
                    }
                    MusAction::DealHand(player_id, mano) => {
                        self.hands[player_id] = mano.clone();
                    }
                    MusAction::Payoff(couple_id, tantos) => {
                        self.scoreboard[couple_id] += tantos;
                        self.arena_events.push(mus_action);
                    }
                    _ => {
                        self.arena_events.push(mus_action);
                    }
                },
                ArenaMessage::ActionRequested(actions) => self.actions = actions,
            },
            GameEvent::ActionSelected(accion) => {
                let _ = self.to_agent.as_mut().unwrap().try_send(accion);
            }
        }
    }
}

fn setup_arena(strategy: Strategy<LanceGame>) -> impl Stream<Item = ArenaMessage> {
    iced::stream::channel(100, move |sender| async move {
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
            receiver: mpsc::Receiver<Accion>,
            history: Arc<Mutex<Vec<Accion>>>,
        }
        impl AgentGui {
            fn new(
                mut sender: mpsc::Sender<ArenaMessage>,
                history: Arc<Mutex<Vec<Accion>>>,
            ) -> Self {
                let (to_agent, receiver) = mpsc::channel(100);
                let _ = sender.try_send(ArenaMessage::AgentInitialized(to_agent));
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
                self.receiver.next().await.unwrap()
            }
        }
        let lance = strategy.strategy_config.game_config.lance;
        let mut arena = MusArena::new(lance);
        let kibitzer = KibitzerGui::new(sender.clone());
        let action_recorder = ActionRecorder::new();
        let agent_musolver = AgenteMusolver::new(strategy, action_recorder.history());
        let agent_gui = AgentGui::new(sender.clone(), action_recorder.history());

        arena.agents.push(Box::new(agent_gui));
        arena.agents.push(Box::new(agent_musolver.clone()));
        arena.agents.push(Box::new(agent_musolver.clone()));
        arena.agents.push(Box::new(agent_musolver.clone()));
        arena.kibitzers.push(Box::new(kibitzer));
        arena.kibitzers.push(Box::new(action_recorder));
        loop {
            arena.start().await;
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
