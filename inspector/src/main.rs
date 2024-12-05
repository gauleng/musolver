use iced::{Element, Task, Theme};
use screen::{ExplorerEvent, GameAction, GameEvent, LoaderEvent, Screen};

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
    fn new() -> (Self, Task<Message>) {
        let (screen, task) = screen::Loader::new();
        (
            Self {
                screen: Screen::Loader(screen),
            },
            task.map(Message::Loader),
        )
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
                                self.screen = Screen::Explorer(screen::ActionPath::new(strategy));
                            }
                            screen::LoaderAction::OpenGame(strategy) => {
                                let (screen, task) = screen::MusArenaUi::new(strategy.clone());
                                self.screen = Screen::Game(screen);
                                return task.map(Message::Game);
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
                    let action = game.update(game_event);
                    if let Some(GameAction::OpenLoader) = action {
                        let (screen, task) = screen::Loader::new();
                        self.screen = Screen::Loader(screen);
                        return task.map(Message::Loader);
                    }
                }
                Task::none()
            }
        }
    }
}

fn main() -> iced::Result {
    iced::application("Inspector", Inspector::update, Inspector::view)
        .theme(|_| Theme::GruvboxDark)
        .run_with(Inspector::new)
}
