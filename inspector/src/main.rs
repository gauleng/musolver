use std::{cell::RefCell, rc::Rc, sync::mpsc::Sender};

use iced::{
    futures::{channel::mpsc, SinkExt, Stream},
    Element, Task, Theme,
};
use musolver::{
    mus::{
        arena::{ActionRecorder, Agent, AgenteMusolver, Kibitzer, MusAction, MusArena},
        Accion, Lance,
    },
    solver::{LanceGame, Strategy},
    Game,
};
use screen::{ActionPath, ExplorerEvent, Game, GameEvent, Loader, LoaderEvent, Screen};

mod screen;

#[derive(Debug)]
enum Message {
    Loader(LoaderEvent),
    Explorer(ExplorerEvent),
    Game(GameEvent),
}

struct Inspector {
    screen: Screen,
}

impl Inspector {
    fn new() -> Self {
        Self {
            screen: Screen::Loader(Loader::new()),
        }
    }

    fn view(&self) -> Element<Message> {
        match &self.screen {
            Screen::Loader(loader) => loader.view().map(Message::Loader),
            Screen::Explorer(action_path) => action_path.view().map(Message::Explorer),
            Screen::Game(game) => game.view().map(Message::Game),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loader(loading_event) => {
                if let Screen::Loader(loader) = &mut self.screen {
                    let action = loader.update(loading_event);
                    if let Some(a) = action {
                        match a {
                            screen::LoaderAction::OpenExplorer(strategy) => {
                                self.screen = Screen::Explorer(ActionPath::new(strategy));
                            }
                            screen::LoaderAction::OpenGame(strategy) => {
                                self.screen = Screen::Game(Game::new(strategy.clone()));
                                return Task::run(setup_arena(strategy), |m| {
                                    Message::Game(GameEvent::ArenaMessage(m))
                                });
                            }
                        }
                    }
                }
                Task::none()
            }
            Message::Explorer(explorer_event) => {
                if let Screen::Explorer(explorer) = &mut self.screen {
                    explorer.update(explorer_event);
                }
                Task::none()
            }
            Message::Game(game_event) => {
                if let Screen::Game(game) = &mut self.screen {
                    game.update(game_event);
                }
                Task::none()
            }
        }
    }
}

fn setup_arena(strategy: Strategy<LanceGame>) -> impl Stream<Item = MusAction> {
    iced::stream::channel(100, move |sender| async move {
        struct KibitzerGui {
            sender: mpsc::Sender<MusAction>,
        }
        impl KibitzerGui {
            fn new(sender: mpsc::Sender<MusAction>) -> Self {
                Self { sender }
            }
        }
        impl Kibitzer for KibitzerGui {
            fn record(&mut self, _partida_mus: &musolver::mus::PartidaMus, action: MusAction) {
                self.sender.try_send(action);
            }
        }

        struct AgentGui {
            sender: mpsc::Sender<MusAction>,
            history: Rc<RefCell<Vec<Accion>>>,
        }
        impl AgentGui {
            fn new(sender: mpsc::Sender<MusAction>, history: Rc<RefCell<Vec<Accion>>>) -> Self {
                Self { sender, history }
            }
        }
        impl Agent for AgentGui {
            fn actuar(&mut self, partida_mus: &musolver::mus::PartidaMus) -> musolver::mus::Accion {
                let mut lance_game = LanceGame::from_partida_mus(partida_mus, true).unwrap();
                for action in self.history.borrow().iter() {
                    lance_game.act(*action);
                }
                let next_actions = lance_game.actions();
                next_actions[0]
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

impl Default for Inspector {
    fn default() -> Self {
        Self::new()
    }
}

fn main() -> iced::Result {
    iced::application("Inspector", Inspector::update, Inspector::view)
        .theme(|_| Theme::GruvboxDark)
        .run()
}
