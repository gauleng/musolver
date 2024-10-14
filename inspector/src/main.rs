use iced::{Element, Theme};
use screen::{ActionPath, ExplorerEvent, Loader, LoaderEvent, Screen};

mod screen;

#[derive(Debug)]
enum Message {
    Loader(LoaderEvent),
    Explorer(ExplorerEvent),
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
        }
    }
    fn update(&mut self, message: Message) {
        match message {
            Message::Loader(loading_event) => {
                if let Screen::Loader(loader) = &mut self.screen {
                    let action = loader.update(loading_event);
                    if let Some(a) = action {
                        match a {
                            screen::LoaderAction::OpenExplorer(strategy) => {
                                self.screen = Screen::Explorer(ActionPath::new(strategy));
                            }
                        }
                    }
                }
            }
            Message::Explorer(explorer_event) => {
                if let Screen::Explorer(explorer) = &mut self.screen {
                    explorer.update(explorer_event);
                }
            }
        }
    }
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
