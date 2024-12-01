use std::{cell::RefCell, rc::Rc};

use iced::{
    futures::{channel::mpsc, Stream},
    widget::{button, column, container, row, text},
    Alignment, Element, Length, Task,
};
use musolver::{
    mus::{
        arena::{ActionRecorder, Agent, AgenteMusolver, Kibitzer, MusAction, MusArena},
        Accion, Lance, Mano,
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
            },
            task,
        )
    }

    pub fn view(&self) -> Element<GameEvent> {
        let hand_row =
            |hand: String, is_dealer| row![text(hand), text(if is_dealer { "(M)" } else { "" })];

        let hands = container(
            column![
                hand_row(self.hands[0].to_string(), self.dealer == 0),
                row![
                    hand_row(self.hands[1].to_string(), self.dealer == 1).width(100),
                    container(text("Pot: "))
                        .width(Length::Fill)
                        .align_x(Alignment::Center),
                    hand_row(self.hands[3].to_string(), self.dealer == 3).width(100)
                ]
                .align_y(Alignment::Center)
                .height(100),
                hand_row(self.hands[2].to_string(), self.dealer == 2),
            ]
            .align_x(Alignment::Center),
        )
        .center(500);

        let history = column(self.arena_events.iter().map(|mus_action| {
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
        }));

        let actions = row(self.actions.iter().map(|action| {
            button(text(action.to_string()))
                .on_press(GameEvent::ActionSelected(*action))
                .into()
        }));
        column![row![hands, history], actions].into()
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
            history: Rc<RefCell<Vec<Accion>>>,
        }
        impl AgentGui {
            fn new(
                mut sender: mpsc::Sender<ArenaMessage>,
                history: Rc<RefCell<Vec<Accion>>>,
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
        impl Agent for AgentGui {
            fn actuar(&mut self, partida_mus: &musolver::mus::PartidaMus) -> musolver::mus::Accion {
                let mut lance_game = LanceGame::from_partida_mus(partida_mus, true).unwrap();
                for action in self.history.borrow().iter() {
                    lance_game.act(*action);
                }
                let next_actions = lance_game.actions();
                let first_action = next_actions[0];
                let _ = self
                    .sender
                    .try_send(ArenaMessage::ActionRequested(next_actions));
                // loop {
                //     if let Ok(accion) = self.receiver.try_next() {
                //         return accion.unwrap();
                //     }
                //     std::thread::sleep(std::time::Duration::from_millis(10));
                // }
                first_action
            }
        }
        let mut arena = MusArena::new(Some(Lance::Grande));
        let kibitzer = KibitzerGui::new(sender.clone());
        let action_recorder = ActionRecorder::new();
        let agent_musolver = AgenteMusolver::new(strategy, action_recorder.history().clone());
        let agent_gui = AgentGui::new(sender.clone(), action_recorder.history().clone());

        arena.agents.push(Box::new(agent_gui));
        arena.agents.push(Box::new(agent_musolver.clone()));
        arena.agents.push(Box::new(agent_musolver.clone()));
        arena.agents.push(Box::new(agent_musolver.clone()));
        arena.kibitzers.push(Box::new(kibitzer));
        arena.kibitzers.push(Box::new(action_recorder));
        // loop {
        arena.start();
        // }
    })
}
