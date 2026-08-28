use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use musolver::{Cfr, Game};

fn main() {
    let rps = Rps::new();
    let mut cfr = Cfr::new().method(musolver::CfrMethod::FsiCfr);

    cfr.train(&rps, 10000);

    let strategy1: Vec<(_, _)> = rps
        .actions()
        .into_iter()
        .zip(cfr.nodes().get(&0).unwrap().get_average_strategy())
        .collect();
    let strategy2: Vec<(_, _)> = rps
        .actions()
        .into_iter()
        .zip(cfr.nodes().get(&1).unwrap().get_average_strategy())
        .collect();
    println!("Strategy player 1: {strategy1:?}");
    println!("Strategy player 2: {strategy2:?}");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

    fn actions(&self) -> Vec<RpsAction> {
        vec![RpsAction::Rock, RpsAction::Paper, RpsAction::Scissors]
    }
}

impl Game for Rps {
    type InfoSet = usize;

    const N_PLAYERS: usize = 2;

    fn utility(&self, player: usize) -> f64 {
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
        if player == 0 { payoff } else { -payoff }
    }

    fn info_set(&self, player: usize) -> usize {
        player
    }

    fn node_key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.history.hash(&mut hasher);
        hasher.finish()
    }

    fn current_node(&self) -> musolver::NodeType {
        self.turn.map_or_else(
            || musolver::NodeType::Terminal,
            |turn| musolver::NodeType::Player(turn, self.actions().len()),
        )
    }

    fn act(&self, action_idx: usize) -> Self {
        let a = self.actions()[action_idx];
        let mut new_game = self.clone();
        new_game.history.push(a);
        new_game.turn = match new_game.turn {
            Some(0) => Some(1),
            _ => None,
        };
        new_game
    }

    fn chance_sample(&self) -> Self {
        todo!()
    }

    fn chance_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        std::iter::empty()
    }
}
