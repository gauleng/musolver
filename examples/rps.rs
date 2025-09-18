use musolver::{Cfr, Game};

fn main() {
    let mut rps = Rps::new();
    let mut cfr = Cfr::new();

    cfr.train(
        &mut rps,
        musolver::CfrMethod::FsiCfr,
        1000,
        |_player, _utility| {},
    );

    let strategy1 = cfr.nodes()["0"].get_average_strategy();
    let strategy2 = cfr.nodes()["1"].get_average_strategy();
    println!("Strategy player 1: {strategy1:?}");
    println!("Strategy player 2: {strategy2:?}");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RpsAction {
    Rock,
    Paper,
    Scissors,
}

#[derive(Debug, Clone)]
struct Rps {
    history: Vec<RpsAction>,
    turn: Option<usize>,
}

impl Rps {
    fn new() -> Rps {
        Rps {
            history: vec![],
            turn: Some(0),
        }
    }
}

impl Game for Rps {
    type Action = RpsAction;

    const N_PLAYERS: usize = 2;

    fn utility(&mut self, player: usize) -> f64 {
        let (action1, action2) = (&self.history[0], &self.history[1]);
        let payoff = match (action1, action2) {
            (RpsAction::Rock, RpsAction::Scissors) => 1.,
            (RpsAction::Rock, RpsAction::Paper) => -1.,
            (RpsAction::Paper, RpsAction::Scissors) => -1.,
            (RpsAction::Paper, RpsAction::Rock) => 1.,
            (RpsAction::Scissors, RpsAction::Rock) => -1.,
            (RpsAction::Scissors, RpsAction::Paper) => 1.,
            _ => 0.,
        };
        if player == 0 {
            payoff
        } else {
            -payoff
        }
    }

    fn info_set_str(&self, player: usize) -> String {
        player.to_string()
    }

    fn history_str(&self) -> String {
        self.history
            .iter()
            .map(|action| format!("{action:?}"))
            .collect::<Vec<String>>()
            .join(",")
    }

    fn actions(&self) -> Vec<Self::Action> {
        vec![RpsAction::Rock, RpsAction::Paper, RpsAction::Scissors]
    }

    fn current_player(&self) -> musolver::NodeType {
        self.turn.map_or_else(
            || musolver::NodeType::Terminal,
            |turn| musolver::NodeType::Player(turn),
        )
    }

    fn act(&mut self, a: Self::Action) {
        self.history.push(a);
        self.turn = match self.turn {
            Some(0) => Some(1),
            Some(1) => None,
            _ => None,
        };
    }

    fn new_random(&mut self) {
        todo!()
    }

    fn new_iter<F>(&mut self, _f: F)
    where
        F: FnMut(&mut Self, f64),
    {
        todo!()
    }
}
