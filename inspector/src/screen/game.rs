use iced::{widget::button, Element};
use musolver::{
    mus::arena::MusAction,
    solver::{LanceGame, Strategy},
};

#[derive(Debug, Clone)]
pub enum GameEvent {
    StartGame,
    ArenaMessage(MusAction),
}

pub struct Game {}

impl Game {
    pub fn new(strategy: Strategy<LanceGame>) -> Self {
        Self {}
    }

    pub fn view(&self) -> Element<GameEvent> {
        let button = button("Start game").on_press(GameEvent::StartGame);
        button.into()
    }

    pub fn update(&mut self, message: GameEvent) {
        match message {
            GameEvent::StartGame => todo!(),
            GameEvent::ArenaMessage(mus_action) => match mus_action {
                MusAction::GameStart(_) => {
                    println!("Game starts!");
                }
                MusAction::DealHand(_, mano) => {
                    println!("Deal hand! {mano}");
                }
                MusAction::LanceStart(lance) => {
                    println!("Lance start: {lance:?}");
                }
                MusAction::PlayerAction(_, accion) => {
                    println!("Player action: {accion}");
                }
                MusAction::Payoff(_, _) => {
                    println!("Payoff");
                }
            },
        }
    }
}
