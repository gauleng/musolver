use iced::{
    widget::{button, column, container, row, scrollable, text, text_input},
    Element,
    Length::{Fill, Shrink},
    Task,
};
use musolver::solver::{BancoEstrategias, LanceGame, Strategy, StrategyConfig};

#[derive(Debug, Clone)]
pub enum LoaderEvent {
    ListStrategies(Vec<(String, StrategyConfig)>),
    SearchText(String),
    LoadStrategy(String),
    PlayStrategy(String),
}

pub enum LoaderAction {
    OpenExplorer(Strategy<LanceGame>),
    OpenGame(Strategy<LanceGame>),
}

pub struct Loader {
    search: String,
    strategies: Vec<(String, StrategyConfig)>,
}

impl Loader {
    pub fn new() -> (Self, Task<LoaderEvent>) {
        (
            Self {
                search: "".to_string(),
                strategies: vec![],
            },
            Task::perform(
                async { BancoEstrategias::find("output") },
                LoaderEvent::ListStrategies,
            ),
        )
    }

    pub fn view(&self) -> Element<LoaderEvent> {
        let search = text_input("Search strategy", &self.search).on_input(LoaderEvent::SearchText);

        let strategy_list: Element<_> = {
            let entries = column(
                self.strategies
                    .iter()
                    .filter(|(path, _)| {
                        self.search.is_empty() || path.to_lowercase().contains(&self.search)
                    })
                    .map(|(path, strategy_config)| {
                        container(row![
                            column![
                                text(path),
                                text("-"),
                                text!(
                                    "{:?} - {} iterations - {:?}",
                                    strategy_config.trainer_config.method,
                                    strategy_config.trainer_config.iterations,
                                    strategy_config.game_config.lance
                                )
                            ]
                            .width(Fill),
                            container(
                                row![
                                    button("Play")
                                        .on_press(LoaderEvent::PlayStrategy(path.to_owned())),
                                    button("Load")
                                        .on_press(LoaderEvent::LoadStrategy(path.to_owned())),
                                ]
                                .spacing(5)
                            )
                            .center_y(Shrink)
                        ])
                        .style(container::rounded_box)
                        .width(Fill)
                        .padding(20)
                        .into()
                    }),
            )
            .spacing(10);
            scrollable(entries).height(Fill).spacing(10).into()
        };

        container(column![search, strategy_list].spacing(10))
            .padding(10)
            .into()
    }

    pub fn update(&mut self, message: LoaderEvent) -> Option<LoaderAction> {
        match message {
            LoaderEvent::SearchText(text) => {
                self.search = text;
                None
            }
            LoaderEvent::LoadStrategy(path) => {
                let strategy = Strategy::from_file(path);
                Some(LoaderAction::OpenExplorer(strategy.unwrap()))
            }
            LoaderEvent::PlayStrategy(path) => {
                let strategy = Strategy::from_file(path);
                Some(LoaderAction::OpenGame(strategy.unwrap()))
            }
            LoaderEvent::ListStrategies(list) => {
                self.strategies = list;
                None
            }
        }
    }
}
