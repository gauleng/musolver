use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};

use super::GameError;

/// Node of the CFR algorithm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node<A> {
    regret_sum: Vec<f64>,
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

/// Trait implemented by games that can be trained with the CFR algorithm, for players identified
/// with P and possible actions A.
pub trait Game<P, A> {
    /// Number of players of the game.
    fn num_players(&self) -> usize;

    /// Identifier for the player in position idx.
    fn player_id(&self, idx: usize) -> P;

    /// Utility function for the player P after the actions considered in the history slice.
    fn utility(&self, player: P) -> f64;

    /// Sring representation of the information set for player P after the actions considered in
    /// the history slice.
    fn info_set_str(&self, player: P) -> String;

    /// Actions available in the current state of the game.
    fn actions(&self) -> Option<Vec<A>>;

    /// Indicates if the current state of the game is terminal.
    fn is_terminal(&self) -> bool;

    /// Player to play in the current state of the game.
    fn current_player(&self) -> Option<P>;

    /// Advance the state with the given action for the current player.
    fn act(&mut self, a: A);

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
}

impl FromStr for CfrMethod {
    type Err = GameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "cfr" => Ok(CfrMethod::Cfr),
            "cfr-plus" => Ok(CfrMethod::CfrPlus),
            "chance-sampling" => Ok(CfrMethod::ChanceSampling),
            "external-sampling" => Ok(CfrMethod::ExternalSampling),
            _ => Err(GameError::InvalidCfrMethod(s.to_owned())),
        }
    }
}

/// Implementation of the CFR algorithm.
#[derive(Debug, Clone)]
pub struct Cfr<A> {
    nodes: HashMap<String, Node<A>>,
}

impl<A> Cfr<A> {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn train<G, P, F>(
        &mut self,
        game: &mut G,
        cfr_method: CfrMethod,
        iterations: usize,
        mut iteration_callback: F,
    ) where
        G: Game<P, A> + Clone,
        P: Eq + Copy,
        A: Eq + Copy,
        F: FnMut(&usize, &[f64]),
    {
        let mut util = vec![0.; game.num_players()];
        for i in 0..iterations {
            for (player_idx, u) in util.iter_mut().enumerate() {
                let player_id = game.player_id(player_idx);
                match cfr_method {
                    CfrMethod::Cfr => {
                        game.new_iter(|game, po| {
                            *u += po * self.chance_sampling(game, player_id, 1., po);
                        });
                    }
                    CfrMethod::CfrPlus => todo!(),
                    CfrMethod::ChanceSampling => {
                        game.new_random();
                        *u += self.chance_sampling(game, player_id, 1., 1.);
                    }
                    CfrMethod::ExternalSampling => {
                        game.new_random();
                        *u += self.external_sampling(game, player_id);
                    }
                }
            }
            iteration_callback(&i, &[util[0] / i as f64, util[1] / i as f64]);
        }
    }

    /// Chance sampling CFR algorithm.
    fn chance_sampling<G, P>(&mut self, game: &mut G, player: P, pi: f64, po: f64) -> f64
    where
        G: Game<P, A> + Clone,
        P: Eq + Copy,
        A: Eq + Copy,
    {
        if game.is_terminal() {
            return game.utility(player);
        }
        let current_player = game.current_player().unwrap();
        let actions: Vec<A> = game.actions().unwrap();
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
    fn external_sampling<G, P>(&mut self, game: &mut G, player: P) -> f64
    where
        G: Game<P, A>,
        P: Eq + Copy,
        A: Eq + Copy,
    {
        if game.is_terminal() {
            return game.utility(player);
        }
        let current_player = game.current_player().unwrap();
        let info_set_str = game.info_set_str(current_player);
        let actions: Vec<A> = game.actions().unwrap();
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

    pub fn nodes(&self) -> &HashMap<String, Node<A>> {
        &self.nodes
    }

    pub fn update_strategy(&mut self) {
        self.nodes.values_mut().for_each(|n| {
            n.update_strategy();
        });
    }
}

impl<A> Default for Cfr<A> {
    fn default() -> Self {
        Self::new()
    }
}
