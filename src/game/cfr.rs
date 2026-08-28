use dashmap::DashMap;
use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rayon::ThreadPoolBuilder;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rustc_hash::FxBuildHasher;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::sync::Mutex;
use std::{collections::HashMap, str::FromStr};

use super::{GameError, GameGraph};

/// Node of the CFR algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    // (0..num_actions) regret
    // (num_actions..num_actions*2-1) strategy_sum
    data: Box<[f64]>,
}

impl Node {
    pub fn new(num_actions: usize) -> Self {
        Self {
            data: vec![0.; num_actions * 2].into_boxed_slice(),
        }
    }

    pub fn matched_strategy(&self) -> Vec<f64> {
        let mut strategy: Vec<f64> = self.data[0..(self.data.len() / 2)]
            .iter()
            .map(|&s| s.max(0.))
            .collect();
        let normalizing_sum: f64 = strategy.iter().sum();
        for i in 0..strategy.len() {
            if normalizing_sum > 0. {
                strategy[i] /= normalizing_sum;
            } else {
                strategy[i] = 1. / strategy.len() as f64;
            }
        }
        strategy
    }

    pub fn get_average_strategy(&self) -> Vec<f64> {
        let strategy_sum = self.strategy_sum();
        let normalizing_sum: f64 = strategy_sum.iter().sum();
        if normalizing_sum > 0. {
            strategy_sum.iter().map(|s| s / normalizing_sum).collect()
        } else {
            vec![1. / strategy_sum.len() as f64; strategy_sum.len()]
        }
    }

    pub fn update_strategy_sum(&mut self, weight: f64, strategy: &[f64]) {
        std::iter::zip(self.strategy_sum_mut(), strategy).for_each({
            |(a, &b)| {
                *a += weight * b;
            }
        });
    }

    pub fn get_random_action(&self, strategy: &[f64]) -> usize {
        let dist = WeightedIndex::new(strategy).unwrap();
        dist.sample(&mut rand::thread_rng())
    }

    fn regret_sum_mut(&mut self) -> &mut [f64] {
        let mid = self.data.len() / 2;
        &mut self.data[0..mid]
    }

    fn strategy_sum(&self) -> &[f64] {
        let mid = self.data.len() / 2;
        &self.data[mid..]
    }

    fn strategy_sum_mut(&mut self) -> &mut [f64] {
        let mid = self.data.len() / 2;
        &mut self.data[mid..]
    }
}

#[derive(Debug, Default)]
struct CfrData {
    reach_player: f64,
    reach_opponent: f64,
    utility: f64,
    strategy: Vec<f64>,
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
/// game uses as the key identifying an information set.
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
///    # fn node_key(&self) -> u64 {
///    #     self.history.len() as u64
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
///    let mut cfr = Cfr::new()
///                     .method(musolver::CfrMethod::FsiCfr);
///
///    cfr.train(
///        &mut rps,
///        10000,
///    );
/// ```
///
/// It supports different algorithms that can be set by calling method(). Linear discount is applied
/// to the regret and strategy accumulators every discount_round_size iterations.
///
/// Moreover, serial and parallel execution are supported. Parallel execution has two modes of
/// operation. The first one is iteration-level, set by calling parallel_iterations(workers), and it
/// is suitable for imperfect recall games with chance nodes at the roog of the game tree. The
/// second mode is branch-level, configured by parallel_branches(workers, max_depth), that creates
/// new threads each time the game tree is branched, up to a maximum depth.
pub struct Cfr<G: Game + Send + Sync> {
    nodes: DashMap<G::InfoSet, Node, FxBuildHasher>,
    discount_round_size: usize,
    method: CfrMethod,
    on_progress: Option<ProgressFn>,
    utility: Vec<f64>,
    mode: ExecutionMode,
}

type ProgressFn = Box<dyn Fn(usize, Vec<f64>) + Send + Sync>;

#[derive(PartialEq, Eq)]
enum ExecutionMode {
    Serial,
    ParallelBranches { workers: usize, max_depth: usize },
    ParallelIterations { workers: usize },
}

impl<G: Game + Send + Sync> Cfr<G> {
    /// Creates a new Cfr. By default, it is configured to use CfrMethod::ExternalSampling with
    /// serial execution.
    pub fn new() -> Self {
        let method = CfrMethod::ExternalSampling;
        Self {
            nodes: DashMap::with_hasher(FxBuildHasher),
            discount_round_size: Self::default_discount_round_size(method),
            method,
            on_progress: None,
            utility: vec![0.; G::N_PLAYERS],
            mode: ExecutionMode::Serial,
        }
    }

    fn default_discount_round_size(method: CfrMethod) -> usize {
        match method {
            CfrMethod::Cfr | CfrMethod::CfrPlus => 1,
            CfrMethod::ChanceSampling | CfrMethod::ExternalSampling | CfrMethod::FsiCfr => 100_000,
        }
    }

    /// Sets the CFR method to use in training.
    pub fn method(mut self, m: CfrMethod) -> Self {
        self.discount_round_size = Self::default_discount_round_size(m);
        self.method = m;
        self
    }

    /// Sets a callback function to report the progress of the algorithm. This callback receives the
    /// iteration number and the current estimated expected value.
    pub fn on_progress(mut self, f: impl Fn(usize, Vec<f64>) + Send + Sync + 'static) -> Self {
        self.on_progress = Some(Box::new(f));
        self
    }

    /// Number of iterations to apply linear discount to the regret and strategy accumulators.
    pub fn discount_round_size(mut self, round_size: usize) -> Self {
        self.discount_round_size = round_size;
        self
    }

    /// Average utility estimated after training.
    pub fn utility(&self) -> &[f64] {
        &self.utility
    }

    /// Sets branch-level parallel execution with workers threads. Branching is stopped at
    /// max_depth.
    pub fn parallel_branches(mut self, workers: usize, max_depth: usize) -> Self {
        self.mode = ExecutionMode::ParallelBranches { workers, max_depth };
        self
    }

    /// Sets iteration-level parallel exeuction with workers threads.
    pub fn parallel_iterations(mut self, workers: usize) -> Self {
        self.mode = ExecutionMode::ParallelIterations { workers };
        self
    }

    /// Sets serial execution. It is the default execution mode.
    pub fn serial(mut self) -> Self {
        self.mode = ExecutionMode::Serial;
        self
    }

    pub fn train(&mut self, game: &G, iterations: usize)
    where
        G: Clone,
        G::InfoSet: Send + Sync,
    {
        match self.mode {
            ExecutionMode::Serial => {
                self.run(game, iterations);
            }
            ExecutionMode::ParallelBranches {
                workers,
                max_depth: _,
            }
            | ExecutionMode::ParallelIterations { workers } => {
                let pool = ThreadPoolBuilder::new()
                    .num_threads(workers)
                    .build()
                    .unwrap();
                pool.install(|| {
                    self.run(game, iterations);
                })
            }
        };
        self.utility
            .iter_mut()
            .for_each(|u| *u /= iterations as f64);
    }

    fn run(&mut self, game: &G, iterations: usize)
    where
        G: Clone,
        G::InfoSet: Send + Sync,
    {
        let mut game_graph = GameGraph::new(game);
        let progress = Mutex::new((0usize, vec![0.; G::N_PLAYERS]));
        let num_rounds = iterations / self.discount_round_size;
        let leftover = iterations % self.discount_round_size;
        for round in 0..num_rounds {
            let utility =
                self.run_batch(game, &mut game_graph, self.discount_round_size, &progress);
            self.utility
                .iter_mut()
                .zip(utility)
                .for_each(|(x, y)| *x += y);
            self.discount((round as f64 + 1.) / (round as f64 + 2.));
        }
        if leftover > 0 {
            let utility = self.run_batch(game, &mut game_graph, leftover, &progress);
            self.utility
                .iter_mut()
                .zip(utility)
                .for_each(|(x, y)| *x += y);
        }
    }

    fn run_batch(
        &self,
        game: &G,
        game_graph: &mut GameGraph<G, CfrData>,
        count: usize,
        progress: &Mutex<(usize, Vec<f64>)>,
    ) -> Vec<f64>
    where
        G: Clone,
        G::InfoSet: Send + Sync,
    {
        let sum = |mut a: Vec<f64>, b: Vec<f64>| {
            a.iter_mut().zip(b).for_each(|(x, y)| *x += y);
            a
        };
        if let ExecutionMode::ParallelIterations { workers: _ } = self.mode {
            (0..count)
                .into_par_iter()
                .map(|_| self.run_once(game, None, progress))
                .reduce(|| vec![0.; G::N_PLAYERS], sum)
        } else {
            (0..count)
                .map(|_| self.run_once(game, Some(&mut *game_graph), progress))
                .fold(vec![0.; G::N_PLAYERS], sum)
        }
    }

    fn run_once(
        &self,
        game: &G,
        game_graph: Option<&mut GameGraph<G, CfrData>>,
        progress: &Mutex<(usize, Vec<f64>)>,
    ) -> Vec<f64>
    where
        G: Clone,
        G::InfoSet: Send + Sync,
    {
        let mut utility = vec![0.; G::N_PLAYERS];
        match self.method {
            CfrMethod::Cfr => {
                for (player_idx, u) in utility.iter_mut().enumerate() {
                    *u = self.cfr(game, player_idx, 1., 1.);
                }
            }
            CfrMethod::CfrPlus => {
                todo!();
            }
            CfrMethod::ChanceSampling => {
                for (player_idx, u) in utility.iter_mut().enumerate() {
                    *u = self.chance_sampling(game, player_idx, 1., 1.);
                }
            }
            CfrMethod::ExternalSampling => {
                for (player_idx, u) in utility.iter_mut().enumerate() {
                    *u = self.external_sampling(game, player_idx, 0);
                }
            }
            CfrMethod::FsiCfr => {
                let mut local_graph = None;
                let graph = game_graph.map_or_else(
                    || {
                        local_graph = Some(GameGraph::new(game));
                        local_graph.as_mut().unwrap()
                    },
                    |g| {
                        g.reset();
                        g
                    },
                );
                graph.inflate();
                for (player_idx, u) in utility.iter_mut().enumerate() {
                    *u = self.fsicfr(graph, player_idx);
                }
            }
        }
        if let Some(on_progress) = &self.on_progress {
            let (count, avg) = {
                let mut guard = progress.lock().unwrap();
                guard.0 += 1;
                guard
                    .1
                    .iter_mut()
                    .zip(&utility)
                    .for_each(|(total, u)| *total += u);
                let avg = guard.1.iter().map(|u| u / guard.0 as f64).collect();
                (guard.0, avg)
            };
            on_progress(count, avg);
        }
        utility
    }

    fn discount(&mut self, weight: f64) {
        self.nodes.iter_mut().for_each(|mut value| {
            value.data.iter_mut().for_each(|r| *r *= weight);
        })
    }

    /// Chance sampling CFR algorithm.
    fn cfr(&self, game: &G, player: usize, pi: f64, po: f64) -> f64 {
        match game.current_node() {
            NodeType::Chance => game
                .chance_iter()
                .map(|(new_game, prob)| prob * self.cfr(&new_game, player, pi, po * prob))
                .sum(),
            NodeType::Player(current_player, num_actions) => {
                let info_set = game.info_set(current_player);
                let strategy = self.strategy(&info_set, num_actions);
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

                if current_player == player {
                    let mut node = self
                        .nodes
                        .entry(info_set)
                        .or_insert_with(|| Node::new(num_actions));
                    node.regret_sum_mut()
                        .iter_mut()
                        .zip(util.iter())
                        .for_each(|(r, u)| *r += po * (u - node_util));
                    node.update_strategy_sum(pi, &strategy);
                }

                node_util
            }
            NodeType::Terminal => game.utility(player),
        }
    }
    /// Chance sampling CFR algorithm.
    fn chance_sampling(&self, game: &G, player: usize, pi: f64, po: f64) -> f64 {
        match game.current_node() {
            NodeType::Chance => {
                let new_game = game.chance_sample();
                self.chance_sampling(&new_game, player, pi, po)
            }
            NodeType::Player(current_player, num_actions) => {
                let info_set = game.info_set(current_player);
                let strategy = self.strategy(&info_set, num_actions);
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

                if current_player == player {
                    let mut node = self
                        .nodes
                        .entry(info_set)
                        .or_insert_with(|| Node::new(num_actions));
                    node.regret_sum_mut()
                        .iter_mut()
                        .zip(util.iter())
                        .for_each(|(r, u)| *r += po * (u - node_util));
                    node.update_strategy_sum(pi, &strategy);
                }

                node_util
            }
            NodeType::Terminal => game.utility(player),
        }
    }

    /// External sampling CFR algorithm.
    fn external_sampling(&self, game: &G, player: usize, depth: usize) -> f64
    where
        G::InfoSet: Send + Sync,
    {
        match game.current_node() {
            NodeType::Chance => {
                let new_game = game.chance_sample();
                self.external_sampling(&new_game, player, depth)
            }
            NodeType::Player(current_player, num_actions) => {
                let info_set_str = game.info_set(current_player);
                let strategy = self.strategy(&info_set_str, num_actions);
                if current_player != player {
                    let new_game = {
                        let mut node = match self.nodes.get_mut(&info_set_str) {
                            Some(node) => node,
                            None => self
                                .nodes
                                .entry(info_set_str)
                                .or_insert_with(|| Node::new(num_actions)),
                        };
                        node.update_strategy_sum(1., &strategy);
                        let s = node.get_random_action(&strategy);
                        game.act(s)
                    };
                    return self.external_sampling(&new_game, player, depth);
                }

                let util: Vec<f64> = match self.mode {
                    ExecutionMode::ParallelBranches {
                        workers: _,
                        max_depth,
                    } if depth < max_depth => (0..num_actions)
                        .into_par_iter()
                        .map(|action| {
                            let new_game = game.act(action);
                            self.external_sampling(&new_game, player, depth + 1)
                        })
                        .collect(),
                    _ => (0..num_actions)
                        .map(|action| {
                            let new_game = game.act(action);
                            self.external_sampling(&new_game, player, depth)
                        })
                        .collect(),
                };
                let node_util = std::iter::zip(&util, strategy).map(|(u, s)| u * s).sum();
                let mut node = match self.nodes.get_mut(&info_set_str) {
                    Some(node) => node,
                    None => self
                        .nodes
                        .entry(info_set_str)
                        .or_insert_with(|| Node::new(num_actions)),
                };
                node.regret_sum_mut()
                    .iter_mut()
                    .zip(util.iter())
                    .for_each(|(r, u)| *r += u - node_util);
                node_util
            }
            NodeType::Terminal => game.utility(player),
        }
    }

    fn fsicfr(&self, game_graph: &mut GameGraph<G, CfrData>, player: usize) -> f64
    where
        G: Clone,
    {
        game_graph.node_mut(0).data_mut().reach_player = 1.;
        game_graph.node_mut(0).data_mut().reach_opponent = 1.;
        let order = game_graph.order().to_vec();
        for &idx in &order {
            let game_node = &mut game_graph.node(idx);
            let game = &mut game_node.game();
            match game.current_node() {
                NodeType::Player(current_player, num_actions) => {
                    let node = self
                        .nodes
                        .entry(game.info_set(current_player))
                        .or_insert_with(|| Node::new(num_actions));
                    game_graph.node_mut(idx).data_mut().strategy = node.matched_strategy();
                    for i in 0..game_graph.node(idx).data().strategy.len() {
                        let s = game_graph.node_mut(idx).data_mut().strategy[i];
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

        for idx in order.into_iter().rev() {
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
                    let mut node = self.nodes.get_mut(info_set_str).unwrap();

                    let utility: Vec<f64> = game_graph
                        .node(idx)
                        .children()
                        .iter()
                        .map(|child_idx| game_graph.node(*child_idx).data().utility)
                        .collect();
                    game_graph.node_mut(idx).data_mut().utility = game_graph
                        .node(idx)
                        .data()
                        .strategy
                        .iter()
                        .zip(utility.iter())
                        .map(|(s, u)| s * u)
                        .sum();
                    if current_player == player {
                        node.regret_sum_mut()
                            .iter_mut()
                            .zip(utility.iter())
                            .for_each(|(r, u)| {
                                *r += game_graph.node(idx).data().reach_opponent
                                    * (u - game_graph.node(idx).data().utility)
                            });
                        node.update_strategy_sum(
                            game_graph.node(idx).data().reach_player,
                            &game_graph.node(idx).data().strategy,
                        );
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
                    let strategy = {
                        let node = self
                            .nodes
                            .entry(info_set_str)
                            .or_insert_with(|| Node::new(num_actions));
                        node.get_average_strategy()
                    };
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
                    let strategy = {
                        let info_set_str = game.info_set(current_player);
                        let node = self
                            .nodes
                            .entry(info_set_str)
                            .or_insert_with(|| Node::new(num_actions));
                        node.get_average_strategy()
                    };
                    for (action, prob) in strategy.iter().enumerate() {
                        let next_game = game.act(action);
                        self.info_sets_player(&next_game, player, po * prob, info_sets);
                    }
                }
            }
            NodeType::Terminal => {}
        }
    }

    fn strategy(&self, info_set: &G::InfoSet, num_actions: usize) -> Vec<f64> {
        self.nodes
            .get(info_set)
            .map(|node| node.matched_strategy())
            .unwrap_or_else(|| vec![1. / num_actions as f64; num_actions])
    }

    pub fn nodes(&self) -> &DashMap<G::InfoSet, Node, FxBuildHasher> {
        &self.nodes
    }
}

impl<G: Game + Send + Sync> IntoIterator for Cfr<G> {
    type Item = (G::InfoSet, Node);

    type IntoIter = dashmap::iter::OwningIter<G::InfoSet, Node, FxBuildHasher>;

    fn into_iter(self) -> Self::IntoIter {
        self.nodes.into_iter()
    }
}

impl<G: Game + Send + Sync> Default for Cfr<G> {
    fn default() -> Self {
        Self::new()
    }
}
