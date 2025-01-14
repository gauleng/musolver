use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

use super::{GameError, GameGraph};

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

#[derive(Debug, Default)]
struct CfrData {
    reach_player: f64,
    reach_opponent: f64,
    utility: f64,
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
                    let round_size = 1_000_000;
                    let round_number = (1 + (i / round_size)) as f64;
                    let round_weight = round_number / (round_number + 1.);
                    game.new_random();
                    let mut game_graph = GameGraph::new(game.clone());
                    game_graph.inflate();
                    game_graph
                        .nodes()
                        .iter()
                        .filter(|node| !node.game().is_terminal())
                        .for_each(|non_terminal_node| {
                            let info_set_str = non_terminal_node.info_set_str().unwrap();
                            self.nodes
                                .entry(info_set_str.to_string())
                                .or_insert_with(|| Node::new(non_terminal_node.game().actions()));
                        });
                    for (player_idx, u) in util.iter_mut().enumerate() {
                        let player_id = game.player_id(player_idx);
                        *u += self.fsicfr(&mut game_graph, player_id, round_weight);
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
                    let mut new_game = game.clone();
                    new_game.act(*accion);
                    self.external_sampling(&mut new_game, player)
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

            let mut new_game = game.clone();
            new_game.act(*accion);
            self.external_sampling(&mut new_game, player)
        }
    }

    fn fsicfr(
        &mut self,
        game_graph: &mut GameGraph<G, CfrData>,
        player: G::P,
        round_weight: f64,
    ) -> f64
    where
        G::P: Eq + Copy,
        G::A: Copy,
    {
        game_graph.node_mut(0).data_mut().reach_player = 1.;
        game_graph.node_mut(0).data_mut().reach_opponent = 1.;
        for idx in 0..game_graph.num_nodes() {
            let game_node = &mut game_graph.node(idx);
            let lance_game = &mut game_node.game();
            if !lance_game.is_terminal() {
                let current_player = lance_game.current_player().unwrap();
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
            if lance_game.is_terminal() {
                game_graph.node_mut(idx).data_mut().utility = lance_game.utility(player);
            } else {
                let current_player = lance_game.current_player().unwrap();
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
            game_graph.node_mut(idx).data_mut().reach_player = 0.;
            game_graph.node_mut(idx).data_mut().reach_opponent = 0.;
        }
        game_graph.node(0).data().utility
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
