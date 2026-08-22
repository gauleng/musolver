use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
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

/// Type of nodes in the game tree.
#[derive(Debug)]
pub enum NodeType {
    /// Chance node.
    Chance,
    /// Player: player node with the player id to act and the number of available actions
    /// for this player.
    Player(usize, usize),
    /// Terminal: terminal node.
    Terminal,
}

/// Trait implemented by games that can be trained with the CFR algorithm.
///
/// `N_PLAYERS` gives the number of players of the game. Actions are referred to by their
/// index among the ones returned by the concrete type's own `actions()` method (not part of
/// this trait, since it depends on the concrete action type); `InfoSet` is whatever type the
/// game uses as the key identifying an information set — a plain `usize` is enough for games
/// like Rock, Paper, Scissors below, while richer games may need a `String` or similar.
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
///
/// impl Rps {
///    fn actions(&self) -> Vec<RpsAction> {
///        vec![RpsAction::Rock, RpsAction::Paper, RpsAction::Scissors]
///    }
/// }
///
/// use musolver::Game;
///
/// impl Game for Rps {
///    type InfoSet = usize;
///    const N_PLAYERS: usize = 2;
///
///    fn utility(&self, player: usize) -> f64 {
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
///    fn info_set(&self, player: usize) -> usize {
///        player
///    }
///
///    fn current_node(&self) -> musolver::NodeType {
///        self.turn.map_or_else(
///            || musolver::NodeType::Terminal,
///            |turn| musolver::NodeType::Player(turn, self.actions().len()),
///        )
///    }
///
///    fn act(&self, action_idx: usize) -> Self {
///        let a = self.actions()[action_idx];
///        let mut new_game = self.clone();
///        new_game.history.push(a);
///        new_game.turn = match new_game.turn {
///            Some(0) => Some(1),
///            _ => None,
///        };
///        new_game
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
///    # fn chance_sample(&self) -> Self {
///    #     self.clone()
///    # }
///    #
///    # fn chance_iter(&self) -> impl Iterator<Item = (Self, f64)> {
///    #     std::iter::empty()
///    # }
///    // ...rest of implementation
/// }
/// ```
pub trait Game: Sized {
    /// Type used as a key to identify information sets.
    type InfoSet: Eq + Hash;

    /// Number of players of the game.
    const N_PLAYERS: usize;

    /// Utility function for the given player in a terminal node.
    fn utility(&self, player: usize) -> f64;

    /// Key identifying the information set for the current player.
    fn info_set(&self, player: usize) -> Self::InfoSet;

    fn node_key(&self) -> u64;

    // Returns if the current node is a chance, terminal or player node.
    fn current_node(&self) -> NodeType;

    /// Advance the state with the given action index for the current player.
    fn act(&self, action_idx: usize) -> Self;

    /// Picks a random action in chance nodes.
    fn chance_sample(&self) -> Self;

    /// Returns an iterator for all available actions in chance nodes.
    fn chance_iter(&self) -> impl Iterator<Item = (Self, f64)>;
}

#[derive(
    Debug,
    Copy,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Serialize,
    rkyv::Deserialize,
    rkyv::Archive,
)]
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
#[derive(Debug)]
pub struct Cfr<G: Game> {
    nodes: HashMap<G::InfoSet, Node>,
}

impl<G: Game> Cfr<G> {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn train<F>(
        &mut self,
        game: &G,
        cfr_method: CfrMethod,
        iterations: usize,
        mut iteration_callback: F,
    ) where
        G: Clone,
        F: FnMut(&usize, &[f64]),
    {
        let mut util = vec![0.; G::N_PLAYERS];
        let round_size = match cfr_method {
            CfrMethod::Cfr | CfrMethod::CfrPlus => 1,
            CfrMethod::ChanceSampling | CfrMethod::ExternalSampling | CfrMethod::FsiCfr => 100_000,
        };
        let mut game_graph = GameGraph::new(game);
        for i in 0..iterations {
            match cfr_method {
                CfrMethod::Cfr => {
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        *u += self.cfr(game, player_idx, 1., 1.);
                    }
                }
                CfrMethod::CfrPlus => {
                    todo!();
                }
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
                    game_graph.reset();
                    game_graph.inflate();
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        *u += self.fsicfr(&mut game_graph, player_idx);
                    }
                }
            }
            if i > 0 {
                if i.is_multiple_of(round_size) {
                    let block = (i / round_size) as f64;
                    self.discount(block / (block + 1.));
                }
                // if i.is_multiple_of(round_size * 10) {
                //     let exp = self.exploitability(game);
                //     println!("Exploitability: {exp}");
                // }
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
    fn cfr(&mut self, game: &G, player: usize, pi: f64, po: f64) -> f64 {
        match game.current_node() {
            NodeType::Chance => game
                .chance_iter()
                .map(|(new_game, prob)| prob * self.cfr(&new_game, player, pi, po * prob))
                .sum(),
            NodeType::Player(current_player, num_actions) => {
                let info_set_str = game.info_set(current_player);
                let strategy = self
                    .nodes
                    .get(&info_set_str)
                    .map(|node| node.strategy().clone())
                    .unwrap_or_else(|| vec![1. / num_actions as f64; num_actions]);

                let util: Vec<f64> = strategy
                    .iter()
                    .enumerate()
                    .map(|(a, s)| {
                        let new_game = game.act(a);
                        if current_player == player {
                            self.cfr(&new_game, player, pi * s, po)
                        } else {
                            self.cfr(&new_game, player, pi, po * s)
                        }
                    })
                    .collect();
                let node_util = util.iter().zip(strategy.iter()).map(|(u, s)| u * s).sum();

                let node = self
                    .nodes
                    .entry(info_set_str)
                    .or_insert_with(|| Node::new(num_actions));
                if current_player == player {
                    node.regret_sum
                        .iter_mut()
                        .zip(util.iter())
                        .for_each(|(r, u)| *r += po * (u - node_util));
                    node.update_strategy_sum(pi);
                    node.update_strategy();
                }

                node_util
            }
            NodeType::Terminal => game.utility(player),
        }
    }
    /// Chance sampling CFR algorithm.
    fn chance_sampling(&mut self, game: &G, player: usize, pi: f64, po: f64) -> f64 {
        match game.current_node() {
            NodeType::Chance => {
                let new_game = game.chance_sample();
                self.chance_sampling(&new_game, player, pi, po)
            }
            NodeType::Player(current_player, num_actions) => {
                let info_set_str = game.info_set(current_player);
                let strategy = self
                    .nodes
                    .get(&info_set_str)
                    .map(|node| node.strategy().clone())
                    .unwrap_or_else(|| vec![1. / num_actions as f64; num_actions]);

                let util: Vec<f64> = strategy
                    .iter()
                    .enumerate()
                    .map(|(a, s)| {
                        let new_game = game.act(a);
                        if current_player == player {
                            self.chance_sampling(&new_game, player, pi * s, po)
                        } else {
                            self.chance_sampling(&new_game, player, pi, po * s)
                        }
                    })
                    .collect();
                let node_util = util.iter().zip(strategy.iter()).map(|(u, s)| u * s).sum();

                let node = self
                    .nodes
                    .entry(info_set_str)
                    .or_insert_with(|| Node::new(num_actions));
                if current_player == player {
                    node.regret_sum
                        .iter_mut()
                        .zip(util.iter())
                        .for_each(|(r, u)| *r += po * (u - node_util));
                    node.update_strategy_sum(pi);
                    node.update_strategy();
                }

                node_util
            }
            NodeType::Terminal => game.utility(player),
        }
    }

    /// External sampling CFR algorithm.
    fn external_sampling(&mut self, game: &G, player: usize) -> f64 {
        match game.current_node() {
            NodeType::Chance => {
                let new_game = game.chance_sample();
                self.external_sampling(&new_game, player)
            }
            NodeType::Player(current_player, num_actions) => {
                let info_set_str = game.info_set(current_player);
                if current_player == player {
                    let util: Vec<f64> = (0..num_actions)
                        .map(|action| {
                            let new_game = game.act(action);
                            self.external_sampling(&new_game, player)
                        })
                        .collect();
                    let node = match self.nodes.get_mut(&info_set_str) {
                        Some(node) => node,
                        None => self
                            .nodes
                            .entry(info_set_str)
                            .or_insert_with(|| Node::new(num_actions)),
                    };
                    let strategy = node.update_strategy();

                    let node_util = std::iter::zip(&util, strategy).map(|(u, s)| u * s).sum();
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
                            .entry(info_set_str)
                            .or_insert_with(|| Node::new(num_actions)),
                    };

                    node.update_strategy();
                    node.update_strategy_sum(1.);
                    let s = node.get_random_action();
                    let new_game = game.act(s);
                    self.external_sampling(&new_game, player)
                }
            }
            NodeType::Terminal => game.utility(player),
        }
    }

    fn fsicfr(
        &mut self,
        game_graph: &mut GameGraph<G, CfrData>,
        player: usize,
        //round_weight: f64,
    ) -> f64
    where
        G: Clone,
    {
        game_graph.node_mut(0).data_mut().reach_player = 1.;
        game_graph.node_mut(0).data_mut().reach_opponent = 1.;
        for idx in 0..game_graph.num_nodes() {
            let game_node = &mut game_graph.node(idx);
            let game = &mut game_node.game();
            match game.current_node() {
                NodeType::Player(current_player, num_actions) => {
                    let info_set_str = game_node
                        .info_set_str()
                        .expect("InfoSet must be valid in non terminal nodes.");
                    let node = match self.nodes.get(info_set_str) {
                        Some(node) => node,
                        None => self
                            .nodes
                            .entry(game.info_set(current_player))
                            .or_insert_with(|| Node::new(num_actions)),
                    };
                    let strategy = node.strategy();
                    for (i, s) in strategy.iter().enumerate() {
                        let child_idx = game_graph.node(idx).children()[i];
                        let indices = [idx, child_idx];
                        let [parent, child] =
                            unsafe { game_graph.nodes_mut().get_disjoint_unchecked_mut(indices) };

                        if current_player == player {
                            child.data_mut().reach_player += s * parent.data().reach_player;
                            child.data_mut().reach_opponent += parent.data().reach_opponent;
                        } else {
                            child.data_mut().reach_player += parent.data().reach_player;
                            child.data_mut().reach_opponent += s * parent.data().reach_opponent;
                        }
                    }
                }
                NodeType::Chance => {
                    let child_idx = game_graph.node(idx).children()[0];
                    let indices = [idx, child_idx];
                    let [parent, child] =
                        unsafe { game_graph.nodes_mut().get_disjoint_unchecked_mut(indices) };
                    child.data_mut().reach_player += parent.data().reach_player;
                    child.data_mut().reach_opponent += parent.data().reach_opponent;
                }
                _ => {}
            }
        }

        for idx in (0..game_graph.num_nodes()).rev() {
            let game = &mut game_graph.node_mut(idx).game_mut();
            match game.current_node() {
                NodeType::Terminal => {
                    game_graph.node_mut(idx).data_mut().utility = game.utility(player);
                }
                NodeType::Player(current_player, _) => {
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
                    let indices = [idx, child_idx];
                    let [parent, child] =
                        unsafe { game_graph.nodes_mut().get_disjoint_unchecked_mut(indices) };
                    parent.data_mut().utility = child.data().utility;
                }
            }
            game_graph.node_mut(idx).data_mut().reach_player = 0.;
            game_graph.node_mut(idx).data_mut().reach_opponent = 0.;
        }
        game_graph.node(0).data().utility
    }

    pub fn expected_utility(&self, game: &G) -> Vec<f64> {
        match game.current_node() {
            NodeType::Chance => {
                let mut utility = vec![0.; G::N_PLAYERS];
                for (game, prob) in game.chance_iter() {
                    for (u, v) in utility.iter_mut().zip(self.expected_utility(&game)) {
                        *u += prob * v;
                    }
                }
                utility
            }
            NodeType::Player(current_player, num_actions) => {
                let info_set_str = game.info_set(current_player);
                let strategy = match self.nodes.get(&info_set_str) {
                    Some(node) => node.get_average_strategy(),
                    None => vec![1. / num_actions as f64; num_actions],
                };
                let mut utility = vec![0.; G::N_PLAYERS];
                for (action, prob) in strategy.iter().enumerate() {
                    // Una acción con probabilidad nula no aporta nada a la suma, así que podar su
                    // subárbol da el mismo resultado exacto. Tras entrenar, la estrategia media
                    // es casi determinista y esto descarta la mayor parte del árbol.
                    if *prob == 0. {
                        continue;
                    }
                    let game = game.act(action);
                    for (u, v) in utility.iter_mut().zip(self.expected_utility(&game)) {
                        *u += prob * v;
                    }
                }
                utility
            }
            NodeType::Terminal => {
                Vec::from_iter((0..G::N_PLAYERS).map(|player_idx| game.utility(player_idx)))
            }
        }
    }

    pub fn exploitability(&mut self, game: &G) -> f64
    where
        G: Clone,
    {
        let info_sets = self.info_sets(game);
        let mut br_strategies = HashMap::new();
        br_strategies.reserve(self.nodes().len());

        (0..G::N_PLAYERS)
            .map(|player| self.best_response_value(game, player, &info_sets, &mut br_strategies))
            .sum()
    }

    pub fn best_response_value(
        &mut self,
        game: &G,
        player: usize,
        info_sets: &HashMap<G::InfoSet, Vec<(G, f64)>>,
        br_strategies: &mut HashMap<G::InfoSet, usize>,
    ) -> f64 {
        match game.current_node() {
            NodeType::Chance => game
                .chance_iter()
                .map(|(game, prob)| {
                    prob * self.best_response_value(&game, player, info_sets, br_strategies)
                })
                .sum(),
            NodeType::Player(current_player, num_actions) => {
                let info_set_str = game.info_set(current_player);
                if player == current_player {
                    let action_idx = match br_strategies.get(&info_set_str) {
                        Some(action_idx) => action_idx,
                        None => {
                            let mut action_values = vec![0.; num_actions];
                            if let Some(games) = info_sets.get(&info_set_str) {
                                games.iter().for_each(|(game, po)| {
                                    (0..num_actions).for_each(|action| {
                                        let new_game = game.act(action);
                                        let br = self.best_response_value(
                                            &new_game,
                                            player,
                                            info_sets,
                                            br_strategies,
                                        );
                                        action_values[action] += po * br;
                                    });
                                });
                            }
                            let br_action = action_values
                                .iter()
                                .enumerate()
                                .max_by(|(_, a), (_, b)| a.total_cmp(b))
                                .map(|(idx, _)| idx)
                                .unwrap();

                            br_strategies.entry(info_set_str).or_insert(br_action)
                        }
                    };
                    let game = game.act(*action_idx);
                    self.best_response_value(&game, player, info_sets, br_strategies)
                } else {
                    let node = self
                        .nodes
                        .entry(info_set_str)
                        .or_insert_with(|| Node::new(num_actions));
                    let strategy = node.get_average_strategy();
                    strategy
                        .iter()
                        .enumerate()
                        .map(|(action, prob)| {
                            if *prob == 0. {
                                return 0.;
                            }
                            let game = game.act(action);
                            prob * self.best_response_value(&game, player, info_sets, br_strategies)
                        })
                        .sum()
                }
            }
            NodeType::Terminal => game.utility(player),
        }
    }

    fn info_sets(&mut self, game: &G) -> HashMap<G::InfoSet, Vec<(G, f64)>>
    where
        G: Clone,
    {
        let mut info_sets = HashMap::new();
        info_sets.reserve(self.nodes().len());

        for player in 0..G::N_PLAYERS {
            self.info_sets_player(game, player, 1., &mut info_sets);
        }

        info_sets
    }

    fn info_sets_player(
        &mut self,
        game: &G,
        player: usize,
        po: f64,
        info_sets: &mut HashMap<G::InfoSet, Vec<(G, f64)>>,
    ) where
        G: Clone,
    {
        match game.current_node() {
            NodeType::Chance => {
                game.chance_iter().for_each(|(new_game, prob)| {
                    self.info_sets_player(&new_game, player, po * prob, info_sets);
                });
            }
            NodeType::Player(current_player, num_actions) => {
                if player == current_player {
                    let info_set_str = game.info_set(current_player);
                    let info_set = info_sets
                        .entry(info_set_str)
                        .or_insert_with(|| Vec::with_capacity(500));
                    info_set.push((game.clone(), po));
                }
                if player == current_player {
                    for action in 0..num_actions {
                        let next_game = game.act(action);
                        self.info_sets_player(&next_game, player, po, info_sets);
                    }
                } else {
                    let info_set_str = game.info_set(current_player);
                    let node = self
                        .nodes
                        .entry(info_set_str)
                        .or_insert_with(|| Node::new(num_actions));
                    let strategy = node.get_average_strategy();
                    for (action, prob) in strategy.iter().enumerate() {
                        let next_game = game.act(action);
                        self.info_sets_player(&next_game, player, po * prob, info_sets);
                    }
                }
            }
            NodeType::Terminal => {}
        }
    }

    pub fn nodes(&self) -> &HashMap<G::InfoSet, Node> {
        &self.nodes
    }

    pub fn update_strategy(&mut self) {
        self.nodes.values_mut().for_each(|n| {
            n.update_strategy();
        });
    }
}

impl<G: Game> Default for Cfr<G> {
    fn default() -> Self {
        Self::new()
    }
}
