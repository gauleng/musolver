use arrayvec::ArrayVec;
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
///    #
///    # fn new_iter(&self) -> impl Iterator<Item = (Self, f64)> {
///    #     std::iter::empty()
///    # }
///    #
///    # fn reset(&mut self) {
///    # }
///    // ...rest of implementation
/// }
/// ```
pub trait Game: Sized {
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

    /// Resets the game to its initial state.
    fn reset(&mut self);

    /// Iterates over all available actions in chance nodes.
    fn new_iter(&self) -> impl Iterator<Item = (Self, f64)>;
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
            game.reset();

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

                            if !self.nodes.contains_key(info_set_str) {
                                self.nodes.insert(
                                    info_set_str.to_string(),
                                    Node::new(non_terminal_node.game().actions().len()),
                                );
                            }
                        });
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        *u += self.fsicfr(&mut game_graph, player_idx);
                    }
                }
            }
            let round_size = 1_000_000;
            if i > 0 && i.is_multiple_of(round_size) {
                let block = (i / round_size) as f64;
                self.discount(block / (block + 1.));
                let exp = self.exploitability(game);
                println!("exploitability: {exp}");
            }
            iteration_callback(&i, &util.iter().map(|u| u / i as f64).collect::<Vec<f64>>());
        }
    }

    fn discount(&mut self, weight: f64) {
        for value in self.nodes.values_mut() {
            value.regret_sum.iter_mut().for_each(|r| *r *= weight);
            value.strategy_sum.iter_mut().for_each(|r| *r *= weight);
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
        let node = match self.nodes.get_mut(&info_set_str) {
            Some(node) => node,
            None => self
                .nodes
                .entry(info_set_str.clone())
                .or_insert_with(|| Node::new(actions.len())),
        };
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

        if let Some(node) = self.nodes.get_mut(&info_set_str)
            && current_player == player
        {
            node.regret_sum
                .iter_mut()
                .zip(util.iter())
                .for_each(|(r, u)| *r += po * (u - node_util));
            node.update_strategy_sum(pi);
            node.update_strategy();
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
            let node = match self.nodes.get_mut(&info_set_str) {
                Some(node) => node,
                None => self
                    .nodes
                    .entry(info_set_str.clone())
                    .or_insert_with(|| Node::new(actions.len())),
            };
            let strategy = node.update_strategy();

            let node_util = util.iter().zip(strategy.iter()).map(|(u, s)| u * s).sum();
            node.regret_sum
                .iter_mut()
                .zip(util.iter())
                .for_each(|(r, u)| *r += u - node_util);
            node_util
        } else {
            let node = match self.nodes.get_mut(&info_set_str) {
                Some(node) => node,
                None => self
                    .nodes
                    .entry(info_set_str.clone())
                    .or_insert_with(|| Node::new(actions.len())),
            };

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
        //round_weight: f64,
    ) -> f64
    where
        G: Game + Clone,
        G::Action: Copy,
    {
        game_graph.node_mut(0).data_mut().reach_player = 1.;
        game_graph.node_mut(0).data_mut().reach_opponent = 1.;
        for idx in 0..game_graph.num_nodes() {
            let game_node = &mut game_graph.node(idx);
            let game = &mut game_node.game();
            match game.current_player() {
                NodeType::Player(current_player) => {
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
                NodeType::Chance => {
                    let child_idx = game_graph.node(idx).children()[0];
                    game_graph.node_mut(child_idx).data_mut().reach_player +=
                        game_graph.node(idx).data().reach_player;
                    game_graph.node_mut(child_idx).data_mut().reach_opponent +=
                        game_graph.node(idx).data().reach_opponent;
                }
                _ => {}
            }
        }

        for idx in (0..game_graph.num_nodes()).rev() {
            let game = &mut game_graph.node_mut(idx).game_mut();
            match game.current_player() {
                NodeType::Terminal => {
                    game_graph.node_mut(idx).data_mut().utility = game.utility(player);
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
                                *r += game_graph.node(idx).data().reach_opponent
                                    * (u - game_graph.node(idx).data().utility)
                            });
                        node.update_strategy_sum(game_graph.node(idx).data().reach_player);
                        node.update_strategy();
                    }
                }
                NodeType::Chance => {
                    let child_idx = game_graph.node(idx).children()[0];
                    game_graph.node_mut(idx).data_mut().utility =
                        game_graph.node_mut(child_idx).data_mut().utility;
                }
            }
            game_graph.node_mut(idx).data_mut().reach_player = 0.;
            game_graph.node_mut(idx).data_mut().reach_opponent = 0.;
        }
        game_graph.node(0).data().utility
    }

    pub fn expected_utility<G>(&mut self, game: &G) -> Vec<f64>
    where
        G: Game + Clone,
        G::Action: Eq + Copy,
    {
        match game.current_player() {
            NodeType::Chance => {
                game.new_iter()
                    .fold(vec![0.; G::N_PLAYERS], |accum, (mut game, prob)| {
                        std::iter::zip(accum, self.expected_utility(&mut game))
                            .map(|(a, b)| a + prob * b)
                            .collect()
                    })
            }
            NodeType::Player(current_player) => {
                let actions = game.actions();
                let info_set_str = game.info_set_str(current_player);
                let node = self
                    .nodes
                    .entry(info_set_str.clone())
                    .or_insert_with(|| Node::new(actions.len()));
                let strategy = node.get_average_strategy();
                actions
                    .iter()
                    .map(|action| {
                        let mut game = game.clone();
                        game.act(*action);
                        game
                    })
                    .zip(strategy)
                    .fold(vec![0.; G::N_PLAYERS], |accum, (mut game, prob)| {
                        std::iter::zip(accum, self.expected_utility(&mut game))
                            .map(|(a, b)| a + prob * b)
                            .collect()
                    })
            }
            NodeType::Terminal => {
                Vec::from_iter((0..G::N_PLAYERS).map(|player_idx| game.clone().utility(player_idx)))
            }
        }
    }

    pub fn exploitability<G>(&mut self, game: &mut G) -> f64
    where
        G: Game + Clone,
        G::Action: Eq + Copy,
    {
        let info_sets = self.info_sets(game);
        let mut br_strategies = HashMap::new();

        (0..G::N_PLAYERS)
            .map(|player| self.best_response_value(game, player, &info_sets, &mut br_strategies))
            .sum()
    }

    pub fn best_response_value<G>(
        &mut self,
        game: &mut G,
        player: usize,
        info_sets: &HashMap<String, Vec<(G, f64)>>,
        br_strategies: &mut HashMap<String, usize>,
    ) -> f64
    where
        G: Game + Clone,
        G::Action: Eq + Copy,
    {
        match game.current_player() {
            NodeType::Chance => game
                .new_iter()
                .map(|(mut game, prob)| {
                    prob * self.best_response_value(&mut game, player, info_sets, br_strategies)
                })
                .sum(),
            NodeType::Player(current_player) => {
                let actions = game.actions();
                let info_set_str = game.info_set_str(current_player);
                if player == current_player {
                    if br_strategies.get(&info_set_str).is_none() {
                        let mut action_values = vec![0.; game.actions().len()];
                        if let Some(games) = info_sets.get(&info_set_str) {
                            games.iter().for_each(|(game, po)| {
                                game.actions().iter().enumerate().for_each(|(idx, action)| {
                                    let mut new_game = game.clone();
                                    new_game.act(*action);
                                    let br = self.best_response_value(
                                        &mut new_game,
                                        player,
                                        info_sets,
                                        br_strategies,
                                    );
                                    action_values[idx] += po * br;
                                });
                            });
                        }
                        let br_action = action_values
                            .iter()
                            .enumerate()
                            .max_by(|(_, a), (_, b)| a.total_cmp(b))
                            .map(|(idx, _)| idx)
                            .unwrap();

                        br_strategies.insert(info_set_str.clone(), br_action);
                    }
                    if let Some(action_idx) = br_strategies.get_mut(&info_set_str) {
                        let best_action = actions[*action_idx];
                        let mut game = game.clone();
                        game.act(best_action);
                        self.best_response_value(&mut game, player, info_sets, br_strategies)
                    } else {
                        0.
                    }
                } else {
                    let node = self
                        .nodes
                        .entry(info_set_str.clone())
                        .or_insert_with(|| Node::new(actions.len()));
                    let strategy = node.get_average_strategy();
                    std::iter::zip(actions, strategy)
                        .map(|(action, prob)| {
                            if prob == 0. {
                                return 0.;
                            }
                            let mut game = game.clone();
                            game.act(action);
                            prob * self.best_response_value(
                                &mut game,
                                player,
                                info_sets,
                                br_strategies,
                            )
                        })
                        .sum()
                }
            }
            NodeType::Terminal => return game.utility(player),
        }
    }

    fn info_sets<G>(&mut self, game: &mut G) -> HashMap<String, Vec<(G, f64)>>
    where
        G: Game + Clone,
        G::Action: Eq + Copy,
    {
        let mut info_sets = HashMap::new();

        for player in 0..G::N_PLAYERS {
            self.info_sets_player(game, player, 1., &mut info_sets);
        }

        info_sets
    }

    fn info_sets_player<G>(
        &mut self,
        game: &mut G,
        player: usize,
        po: f64,
        info_sets: &mut HashMap<String, Vec<(G, f64)>>,
    ) where
        G: Game + Clone,
        G::Action: Eq + Copy,
    {
        match game.current_player() {
            NodeType::Chance => {
                game.new_iter().for_each(|(mut game, prob)| {
                    self.info_sets_player(&mut game, player, po * prob, info_sets);
                });
            }
            NodeType::Player(current_player) => {
                if player == current_player {
                    let info_set_str = game.info_set_str(current_player);
                    let info_set = info_sets.entry(info_set_str).or_insert_with(|| vec![]);
                    info_set.push((game.clone(), po));
                }
                let actions = game.actions();
                if player == current_player {
                    for action in actions {
                        let mut next_game = game.clone();
                        next_game.act(action);
                        self.info_sets_player(&mut next_game, player, po, info_sets);
                    }
                } else {
                    let n_actions = actions.len();
                    let info_set_str = game.info_set_str(current_player);
                    let node = self
                        .nodes
                        .entry(info_set_str)
                        .or_insert_with(|| Node::new(n_actions));
                    let strategy = node.get_average_strategy();
                    for (action, prob) in std::iter::zip(actions, strategy) {
                        let mut next_game = game.clone();
                        next_game.act(action);
                        self.info_sets_player(&mut next_game, player, po * prob, info_sets);
                    }
                }
            }
            NodeType::Terminal => {}
        }
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
