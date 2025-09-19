use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

use super::{GameError, GameGraph};

/// Node of the CFR algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub regret_sum: Vec<f64>,
    strategy: Vec<f64>,
    strategy_sum: Vec<f64>,
}

impl Node {
    pub fn new(num_actions: usize) -> Self {
        Self {
            regret_sum: vec![0.; num_actions],
            strategy: vec![1. / num_actions as f64; num_actions],
            strategy_sum: vec![0.; num_actions],
        }
    }

    pub fn update_strategy(&mut self) -> &Vec<f64> {
        for i in 0..self.strategy.len() {
            self.strategy[i] = self.regret_sum[i].max(0.);
        }
        let normalizing_sum: f64 = self.strategy.iter().sum();
        for i in 0..self.strategy.len() {
            if normalizing_sum > 0. {
                self.strategy[i] /= normalizing_sum;
            } else {
                self.strategy[i] = 1. / self.strategy.len() as f64;
            }
        }
        &self.strategy
    }

    pub fn strategy(&self) -> &Vec<f64> {
        &self.strategy
    }

    pub fn get_average_strategy(&self) -> Vec<f64> {
        let normalizing_sum: f64 = self.strategy_sum.iter().sum();
        if normalizing_sum > 0. {
            self.strategy_sum
                .iter()
                .map(|s| s / normalizing_sum)
                .collect()
        } else {
            vec![1. / self.strategy.len() as f64; self.strategy.len()]
        }
    }

    pub fn update_strategy_sum(&mut self, weight: f64) {
        for i in 0..self.strategy.len() {
            self.strategy_sum[i] += weight * self.strategy[i];
        }
    }

    pub fn get_random_action(&self) -> usize {
        let dist = WeightedIndex::new(&self.strategy).unwrap();
        dist.sample(&mut rand::thread_rng())
    }
}

#[derive(Debug, Default)]
struct CfrData {
    reach_player: f64,
    reach_opponent: f64,
    utility: f64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NodeType {
    Chance,
    Player(usize),
    Terminal,
}

/// Trait implemented by games that can be trained with the CFR algorithm.
///
/// There are two bound types, `N_PLAYERS` with the number of players of the game,
/// and `Action`, with the available actions.
///
/// For example, for the Rock, Paper, Scissors game, the following actions are available:
///
///```
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum RpsAction {
///    Rock,
///    Paper,
///    Scissors,
/// }
/// ```
///
/// The game can be implemented as follows:
/// ```
/// # #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// # enum RpsAction {
/// #   Rock,
/// #   Paper,
/// #   Scissors,
/// # }
/// #[derive(Debug, Clone)]
/// struct Rps {
///    history: Vec<RpsAction>,
///    turn: Option<usize>,
/// }
/// use musolver::Game;
///
/// impl Game for Rps {
///    type Action = RpsAction;
///    const N_PLAYERS: usize = 2;
///
///    fn utility(&mut self, player: usize) -> f64 {
///        let (action1, action2) = (&self.history[0], &self.history[1]);
///        let payoff = match (action1, action2) {
///            (RpsAction::Rock, RpsAction::Scissors) => 1.,
///            (RpsAction::Rock, RpsAction::Paper) => -1.,
///            (RpsAction::Paper, RpsAction::Scissors) => -1.,
///            (RpsAction::Paper, RpsAction::Rock) => 1.,
///            (RpsAction::Scissors, RpsAction::Rock) => -1.,
///            (RpsAction::Scissors, RpsAction::Paper) => 1.,
///            _ => 0.,
///        };
///        if player == 0 {
///            payoff
///        } else {
///            -payoff
///        }
///    }
///
///    fn info_set_str(&self, player: usize) -> String {
///        player.to_string()
///    }
///
///    fn actions(&self) -> Vec<Self::Action> {
///        vec![RpsAction::Rock, RpsAction::Paper, RpsAction::Scissors]
///    }
///
///    fn current_player(&self) -> musolver::NodeType {
///        self.turn.map_or_else(
///            || musolver::NodeType::Terminal,
///            |turn| musolver::NodeType::Player(turn),
///        )
///    }
///
///    fn act(&mut self, a: Self::Action) {
///        self.history.push(a);
///        self.turn = match self.turn {
///            Some(0) => Some(1),
///            _ => None,
///        };
///    }
///
///    # fn history_str(&self) -> String {
///    #     self.history
///    #         .iter()
///    #         .map(|action| format!("{action:?}"))
///    #         .collect::<Vec<String>>()
///    #         .join(",")
///    # }
///    #
///    # fn new_random(&mut self) {
///    # }
///
///    # fn new_iter<F>(&mut self, _f: F)
///    # where
///    #     F: FnMut(&mut Self, f64),
///    # {
///    # }
///    // ...rest of implementation
/// }
/// ```
pub trait Game {
    type Action;
    const N_PLAYERS: usize;

    /// Utility function for the player P after the actions considered in the history slice.
    fn utility(&mut self, player: usize) -> f64;

    /// Sring representation of the information set for player P after the actions considered in
    /// the history slice.
    fn info_set_str(&self, player: usize) -> String;

    fn history_str(&self) -> String;

    /// Actions available in the current state of the game.
    fn actions(&self) -> Vec<Self::Action>;

    // Returns if the current node is a chance, terminal or player node.
    fn current_player(&self) -> NodeType;

    /// Advance the state with the given action for the current player.
    fn act(&mut self, a: Self::Action);

    /// Picks a random action in chance nodes.
    fn new_random(&mut self);

    /// Iterates over all available actions in chance nodes.
    fn new_iter<F>(&mut self, f: F)
    where
        F: FnMut(&mut Self, f64);
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum CfrMethod {
    Cfr,
    CfrPlus,
    ChanceSampling,
    ExternalSampling,
    FsiCfr,
}

impl FromStr for CfrMethod {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "cfr" => Ok(CfrMethod::Cfr),
            "cfr-plus" => Ok(CfrMethod::CfrPlus),
            "chance-sampling" => Ok(CfrMethod::ChanceSampling),
            "external-sampling" => Ok(CfrMethod::ExternalSampling),
            "fsi-cfr" => Ok(CfrMethod::FsiCfr),
            _ => Err(GameError::InvalidCfrMethod(s.to_owned())),
        }
    }
}

/// Implementation of the CFR algorithm. It works on types that implement the trait `Game`.
///
/// ```ignore
///    let mut rps = Rps::new();
///    let mut cfr = Cfr::new();
///
///    cfr.train(
///        &mut rps,
///        musolver::CfrMethod::FsiCfr,
///        10000,
///        |_player, _utility| {},
///    );
/// ```
#[derive(Debug, Clone)]
pub struct Cfr {
    nodes: HashMap<String, Node>,
}

impl Cfr {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn train<G, F>(
        &mut self,
        game: &mut G,
        cfr_method: CfrMethod,
        iterations: usize,
        mut iteration_callback: F,
    ) where
        G: Game + Clone,
        G::Action: Eq + Copy,
        F: FnMut(&usize, &[f64]),
    {
        let mut util = vec![0.; G::N_PLAYERS];
        for i in 0..iterations {
            match cfr_method {
                CfrMethod::Cfr => {
                    todo!();
                    // for (player_idx, u) in util.iter_mut().enumerate() {
                    //     game.new_iter(|game, po| {
                    //         *u += po * self.chance_sampling(game, player_idx, 1., po);
                    //     });
                    // }
                }
                CfrMethod::CfrPlus => todo!(),
                CfrMethod::ChanceSampling => {
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        *u += self.chance_sampling(game, player_idx, 1., 1.);
                    }
                }
                CfrMethod::ExternalSampling => {
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        *u += self.external_sampling(game, player_idx);
                    }
                }
                CfrMethod::FsiCfr => {
                    let round_size = 1_000_000;
                    let round_number = (1 + (i / round_size)) as f64;
                    let round_weight = round_number / (round_number + 1.);
                    let mut game_graph = GameGraph::new(game.clone());
                    game_graph.inflate();
                    game_graph
                        .nodes()
                        .iter()
                        .filter(|node| {
                            node.game().current_player() != NodeType::Terminal
                                && node.game().current_player() != NodeType::Chance
                        })
                        .for_each(|non_terminal_node| {
                            let info_set_str = non_terminal_node.info_set_str().unwrap();
                            self.nodes
                                .entry(info_set_str.to_string())
                                .or_insert_with(|| {
                                    Node::new(non_terminal_node.game().actions().len())
                                });
                        });
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        *u += self.fsicfr(&mut game_graph, player_idx, round_weight);
                    }
                }
            }
            iteration_callback(&i, &util.iter().map(|u| u / i as f64).collect::<Vec<f64>>());
        }
    }

    /// Chance sampling CFR algorithm.
    fn chance_sampling<G>(&mut self, game: &mut G, player: usize, pi: f64, po: f64) -> f64
    where
        G: Game + Clone,
        G::Action: Eq + Copy,
    {
        let current_player = match game.current_player() {
            NodeType::Chance => {
                let mut new_game = game.clone();
                new_game.new_random();
                return self.chance_sampling(&mut new_game, player, pi, po);
            }
            NodeType::Player(current_player) => current_player,
            NodeType::Terminal => {
                return game.utility(player);
            }
        };
        let actions: Vec<<G as Game>::Action> = game.actions();
        let info_set_str = game.info_set_str(current_player);
        let node = self
            .nodes
            .entry(info_set_str.clone())
            .or_insert_with(|| Node::new(actions.len()));
        let strategy = node.strategy().clone();

        let util: Vec<f64> = actions
            .iter()
            .zip(strategy.iter())
            .map(|(a, s)| {
                let mut new_game = game.clone();
                new_game.act(*a);
                if current_player == player {
                    self.chance_sampling(&mut new_game, player, pi * s, po)
                } else {
                    self.chance_sampling(&mut new_game, player, pi, po * s)
                }
            })
            .collect();
        let node_util = util.iter().zip(strategy.iter()).map(|(u, s)| u * s).sum();

        if let Some(node) = self.nodes.get_mut(&info_set_str) {
            if current_player == player {
                node.regret_sum
                    .iter_mut()
                    .zip(util.iter())
                    .for_each(|(r, u)| *r += po * (u - node_util));
                node.update_strategy_sum(pi);
                node.update_strategy();
            }
        }

        node_util
    }

    /// External sampling CFR algorithm.
    fn external_sampling<G>(&mut self, game: &mut G, player: usize) -> f64
    where
        G: Game + Clone,
        G::Action: Eq + Copy,
    {
        let current_player = match game.current_player() {
            NodeType::Chance => {
                let mut new_game = game.clone();
                new_game.new_random();
                return self.external_sampling(&mut new_game, player);
            }
            NodeType::Player(current_player) => current_player,
            NodeType::Terminal => {
                return game.utility(player);
            }
        };
        let info_set_str = game.info_set_str(current_player);
        let actions: Vec<<G as Game>::Action> = game.actions();
        if current_player == player {
            let util: Vec<f64> = actions
                .iter()
                .map(|accion| {
                    let mut new_game = game.clone();
                    new_game.act(*accion);
                    self.external_sampling(&mut new_game, player)
                })
                .collect();
            let node = self
                .nodes
                .entry(info_set_str.clone())
                .or_insert_with(|| Node::new(actions.len()));
            let strategy = node.update_strategy();

            let node_util = util.iter().zip(strategy.iter()).map(|(u, s)| u * s).sum();
            node.regret_sum
                .iter_mut()
                .zip(util.iter())
                .for_each(|(r, u)| *r += u - node_util);
            node_util
        } else {
            let node = self
                .nodes
                .entry(info_set_str.clone())
                .or_insert_with(|| Node::new(actions.len()));

            node.update_strategy();
            node.update_strategy_sum(1.);
            let s = node.get_random_action();
            let accion = actions.get(s).unwrap();

            let mut new_game = game.clone();
            new_game.act(*accion);
            self.external_sampling(&mut new_game, player)
        }
    }

    fn fsicfr<G>(
        &mut self,
        game_graph: &mut GameGraph<G, CfrData>,
        player: usize,
        round_weight: f64,
    ) -> f64
    where
        G: Game + Clone,
        G::Action: Copy,
    {
        game_graph.node_mut(0).data_mut().reach_player = 1.;
        game_graph.node_mut(0).data_mut().reach_opponent = 1.;
        for idx in 0..game_graph.num_nodes() {
            let game_node = &mut game_graph.node(idx);
            let lance_game = &mut game_node.game();
            if let NodeType::Player(current_player) = lance_game.current_player() {
                let info_set_str = game_node
                    .info_set_str()
                    .expect("InfoSet must be valid in non terminal nodes.");
                let node = self
                    .nodes
                    .get(info_set_str)
                    .expect("InfoSet should be preloaded before calling fsicfr.");
                let strategy = node.strategy();
                for (i, s) in strategy.iter().enumerate() {
                    let child_idx = game_graph.node(idx).children()[i];

                    if current_player == player {
                        game_graph.node_mut(child_idx).data_mut().reach_player +=
                            s * game_graph.node(idx).data().reach_player;
                        game_graph.node_mut(child_idx).data_mut().reach_opponent +=
                            game_graph.node(idx).data().reach_opponent;
                    } else {
                        game_graph.node_mut(child_idx).data_mut().reach_player +=
                            game_graph.node(idx).data().reach_player;
                        game_graph.node_mut(child_idx).data_mut().reach_opponent +=
                            s * game_graph.node(idx).data().reach_opponent;
                    }
                }
            }
        }

        for idx in (0..game_graph.num_nodes()).rev() {
            let lance_game = &mut game_graph.node_mut(idx).game_mut();
            match lance_game.current_player() {
                NodeType::Terminal => {
                    game_graph.node_mut(idx).data_mut().utility = lance_game.utility(player);
                }
                NodeType::Player(current_player) => {
                    let info_set_str = game_graph
                        .node(idx)
                        .info_set_str()
                        .expect("InfoSet must be valid in non terminal nodes.");
                    let node = self.nodes.get_mut(info_set_str).unwrap();
                    let strategy = node.strategy();

                    let utility: Vec<f64> = game_graph
                        .node(idx)
                        .children()
                        .iter()
                        .map(|child_idx| game_graph.node(*child_idx).data().utility)
                        .collect();
                    game_graph.node_mut(idx).data_mut().utility = strategy
                        .iter()
                        .zip(utility.iter())
                        .map(|(s, u)| s * u)
                        .sum();
                    if current_player == player {
                        node.regret_sum
                            .iter_mut()
                            .zip(utility.iter())
                            .for_each(|(r, u)| {
                                *r += round_weight
                                    * game_graph.node(idx).data().reach_opponent
                                    * (u - game_graph.node(idx).data().utility)
                            });
                        node.update_strategy_sum(
                            round_weight * game_graph.node(idx).data().reach_player,
                        );
                        node.update_strategy();
                    }
                }
                NodeType::Chance => {}
            }
            game_graph.node_mut(idx).data_mut().reach_player = 0.;
            game_graph.node_mut(idx).data_mut().reach_opponent = 0.;
        }
        game_graph.node(0).data().utility
    }

    pub fn nodes(&self) -> &HashMap<String, Node> {
        &self.nodes
    }

    pub fn update_strategy(&mut self) {
        self.nodes.values_mut().for_each(|n| {
            n.update_strategy();
        });
    }
}

impl Default for Cfr {
    fn default() -> Self {
        Self::new()
    }
}
