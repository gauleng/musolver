use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

use super::GameError;

/// Node of the CFR algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node<A> {
    pub regret_sum: Vec<f64>,
    strategy: Vec<f64>,
    strategy_sum: Vec<f64>,
    actions: Vec<A>,
}

impl<A> Node<A> {
    pub fn new(actions: Vec<A>) -> Self {
        let num_actions = actions.len();
        Self {
            regret_sum: vec![0.; num_actions],
            strategy: vec![1. / num_actions as f64; num_actions],
            strategy_sum: vec![0.; num_actions],
            actions,
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

    pub fn actions(&self) -> &[A] {
        &self.actions
    }
}

#[derive(Debug)]
struct GameNode<G> {
    lance_game: G,
    next_nodes: Vec<usize>,
    reach_player: f64,
    reach_opponent: f64,
    utility: f64,
}

#[derive(Debug)]
struct GameGraph<G> {
    node_ids: HashMap<String, usize>,
    last_node_id: usize,
    game_nodes: Vec<GameNode<G>>,
}

impl<G> GameGraph<G>
where
    G: Game + Clone,
    G::A: Copy,
{
    fn new(game: G) -> Self {
        let history_str = game.history_str();
        let node_ids = HashMap::from([(history_str, 0)]);

        Self {
            node_ids,
            last_node_id: 0,
            game_nodes: vec![GameNode {
                lance_game: game,
                next_nodes: vec![],
                reach_player: 1.,
                reach_opponent: 1.,
                utility: 0.,
            }],
        }
    }

    fn inflate(&mut self) {
        let mut game_list = vec![0];
        while !game_list.is_empty() {
            game_list = game_list
                .drain(..)
                .flat_map(|idx| self.next_nodes(idx))
                .collect();
        }
    }

    fn next_nodes(&mut self, idx: usize) -> Vec<usize> {
        let game = &self.game_nodes[idx].lance_game;
        if game.is_terminal() {
            vec![]
        } else {
            let actions = game.actions();
            actions
                .iter()
                .filter_map(|action| {
                    let mut new_game = self.game_nodes[idx].lance_game.clone();
                    new_game.act(*action);
                    let history_str = new_game.history_str();
                    match self.node_ids.get(&history_str) {
                        Some(next_id) => {
                            self.game_nodes[idx].next_nodes.push(*next_id);
                            None
                        }
                        None => {
                            self.last_node_id += 1;
                            self.node_ids.insert(history_str, self.last_node_id);
                            self.game_nodes.push(GameNode {
                                lance_game: new_game,
                                next_nodes: vec![],
                                reach_player: 0.,
                                reach_opponent: 0.,
                                utility: 0.,
                            });
                            self.game_nodes[idx].next_nodes.push(self.last_node_id);
                            Some(self.last_node_id)
                        }
                    }
                })
                .collect()
        }
    }
}

/// Trait implemented by games that can be trained with the CFR algorithm, for players identified
/// with P and possible actions A.
pub trait Game {
    type A;
    type P;

    /// Number of players of the game.
    fn num_players(&self) -> usize;

    /// Identifier for the player in position idx.
    fn player_id(&self, idx: usize) -> Self::P;

    /// Utility function for the player P after the actions considered in the history slice.
    fn utility(&mut self, player: Self::P) -> f64;

    /// Sring representation of the information set for player P after the actions considered in
    /// the history slice.
    fn info_set_str(&self, player: Self::P) -> String;

    fn history_str(&self) -> String;

    /// Actions available in the current state of the game.
    fn actions(&self) -> Vec<Self::A>;

    /// Indicates if the current state of the game is terminal.
    fn is_terminal(&self) -> bool;

    /// Player to play in the current state of the game.
    fn current_player(&self) -> Option<Self::P>;

    /// Advance the state with the given action for the current player.
    fn act(&mut self, a: Self::A);

    /// Takeback the last action and return to the previous state of the game.
    fn takeback(&mut self);

    /// Initializes the game with a random instance. This method is called by the external and
    /// chance sampling methods.
    fn new_random(&mut self);

    /// Iterates all the possible games.
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

/// Implementation of the CFR algorithm.
#[derive(Debug, Clone)]
pub struct Cfr<G: Game> {
    nodes: HashMap<String, Node<G::A>>,
}

impl<G> Cfr<G>
where
    G: Game + Clone,
{
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn train<F>(
        &mut self,
        game: &mut G,
        cfr_method: CfrMethod,
        iterations: usize,
        mut iteration_callback: F,
    ) where
        G::P: Eq + Copy,
        G::A: Eq + Copy,
        F: FnMut(&usize, &[f64]),
    {
        let mut util = vec![0.; game.num_players()];
        for i in 0..iterations {
            match cfr_method {
                CfrMethod::Cfr => {
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        let player_id = game.player_id(player_idx);
                        game.new_iter(|game, po| {
                            *u += po * self.chance_sampling(game, player_id, 1., po);
                        });
                    }
                }
                CfrMethod::CfrPlus => todo!(),
                CfrMethod::ChanceSampling => {
                    game.new_random();
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        let player_id = game.player_id(player_idx);
                        *u += self.chance_sampling(game, player_id, 1., 1.);
                    }
                }
                CfrMethod::ExternalSampling => {
                    game.new_random();
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        let player_id = game.player_id(player_idx);
                        *u += self.external_sampling(game, player_id);
                    }
                }
                CfrMethod::FsiCfr => {
                    game.new_random();
                    let mut game_graph = GameGraph::new(game.clone());
                    game_graph.inflate();
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        let player_id = game.player_id(player_idx);
                        *u += self.fsicfr(&mut game_graph, player_id);
                    }
                }
            }
            iteration_callback(&i, &util.iter().map(|u| u / i as f64).collect::<Vec<f64>>());
        }
    }

    /// Chance sampling CFR algorithm.
    fn chance_sampling(&mut self, game: &mut G, player: <G as Game>::P, pi: f64, po: f64) -> f64
    where
        G::P: Eq + Copy,
        G::A: Eq + Copy,
    {
        if game.is_terminal() {
            return game.utility(player);
        }
        let current_player = game.current_player().unwrap();
        let actions: Vec<<G as Game>::A> = game.actions();
        let info_set_str = game.info_set_str(current_player);
        let node = self
            .nodes
            .entry(info_set_str.clone())
            .or_insert_with(|| Node::new(actions.clone()));
        let strategy = node.strategy().clone();

        let util: Vec<f64> = actions
            .iter()
            .zip(strategy.iter())
            .map(|(a, s)| {
                game.act(*a);
                let u = if current_player == player {
                    self.chance_sampling(game, player, pi * s, po)
                } else {
                    self.chance_sampling(game, player, pi, po * s)
                };
                game.takeback();
                u
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
    fn external_sampling(&mut self, game: &mut G, player: <G as Game>::P) -> f64
    where
        G::P: Eq + Copy,
        G::A: Eq + Copy,
    {
        if game.is_terminal() {
            return game.utility(player);
        }
        let current_player = game.current_player().unwrap();
        let info_set_str = game.info_set_str(current_player);
        let actions: Vec<<G as Game>::A> = game.actions();
        if current_player == player {
            let util: Vec<f64> = actions
                .iter()
                .map(|accion| {
                    game.act(*accion);
                    let u = self.external_sampling(game, player);
                    game.takeback();
                    u
                })
                .collect();
            let node = self
                .nodes
                .entry(info_set_str.clone())
                .or_insert_with(|| Node::new(actions.clone()));
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
                .or_insert_with(|| Node::new(actions.clone()));

            node.update_strategy();
            node.update_strategy_sum(1.);
            let s = node.get_random_action();
            let accion = actions.get(s).unwrap();

            game.act(*accion);
            let util = self.external_sampling(game, player);
            game.takeback();
            util
        }
    }

    fn fsicfr(&mut self, game_graph: &mut GameGraph<G>, player: G::P) -> f64
    where
        G::P: Eq + Copy,
        G::A: Copy,
    {
        game_graph.game_nodes[0].reach_player = 1.;
        game_graph.game_nodes[0].reach_opponent = 1.;
        for idx in 0..game_graph.game_nodes.len() {
            let game_node = &mut game_graph.game_nodes[idx];
            let lance_game = &mut game_node.lance_game;
            if !lance_game.is_terminal() {
                let current_player = lance_game.current_player().unwrap();
                let info_set_str = lance_game.info_set_str(current_player);
                let actions = lance_game.actions();
                let node = self
                    .nodes
                    .entry(info_set_str.clone())
                    .or_insert_with(|| Node::new(actions.clone()));
                let strategy = node.strategy();
                for (i, s) in strategy.iter().enumerate() {
                    let child_idx = game_graph.game_nodes[idx].next_nodes[i];

                    if current_player == player {
                        game_graph.game_nodes[child_idx].reach_player +=
                            s * game_graph.game_nodes[idx].reach_player;
                        game_graph.game_nodes[child_idx].reach_opponent +=
                            game_graph.game_nodes[idx].reach_opponent;
                    } else {
                        game_graph.game_nodes[child_idx].reach_player +=
                            game_graph.game_nodes[idx].reach_player;
                        game_graph.game_nodes[child_idx].reach_opponent +=
                            s * game_graph.game_nodes[idx].reach_opponent;
                    }
                }
            }
        }

        for idx in (0..game_graph.game_nodes.len()).rev() {
            let lance_game = &mut game_graph.game_nodes[idx].lance_game;
            if lance_game.is_terminal() {
                game_graph.game_nodes[idx].utility = lance_game.utility(player);
            } else {
                let current_player = lance_game.current_player().unwrap();
                let info_set_str = lance_game.info_set_str(current_player);
                let node = self.nodes.get_mut(&info_set_str).unwrap();
                let strategy = node.strategy();

                let utility: Vec<f64> = game_graph.game_nodes[idx]
                    .next_nodes
                    .iter()
                    .map(|child_idx| game_graph.game_nodes[*child_idx].utility)
                    .collect();
                game_graph.game_nodes[idx].utility = strategy
                    .iter()
                    .zip(utility.iter())
                    .map(|(s, u)| s * u)
                    .sum();
                if current_player == player {
                    node.regret_sum
                        .iter_mut()
                        .zip(utility.iter())
                        .for_each(|(r, u)| {
                            *r += game_graph.game_nodes[idx].reach_opponent
                                * (u - game_graph.game_nodes[idx].utility)
                        });
                    node.update_strategy_sum(game_graph.game_nodes[idx].reach_player);
                    node.update_strategy();
                }
            }
            game_graph.game_nodes[idx].reach_player = 0.;
            game_graph.game_nodes[idx].reach_opponent = 0.;
        }
        game_graph.game_nodes[0].utility
    }

    pub fn nodes(&self) -> &HashMap<String, Node<G::A>> {
        &self.nodes
    }

    pub fn update_strategy(&mut self) {
        self.nodes.values_mut().for_each(|n| {
            n.update_strategy();
        });
    }
}

impl<G> Default for Cfr<G>
where
    G: Game + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}
