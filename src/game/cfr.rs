use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::ActionNode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    regret_sum: Vec<f64>,
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

pub trait Game<P, A> {
    fn num_players(&self) -> usize;
    fn player_id(&self, idx: usize) -> P;
    fn utility(&self, player: P, history: &[A]) -> f64;
    fn info_set_str(&self, player: P, history: &[A]) -> String;
    fn new_random(&mut self);
    fn new_iter<F>(&mut self, f: F)
    where
        F: FnMut(&Self, f64);
}

#[derive(Debug, Clone)]
pub struct Cfr<A> {
    history: Vec<A>,
    nodes: HashMap<String, Node>,
}

impl<A> Cfr<A>
where
    A: Eq + Copy,
{
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            nodes: HashMap::new(),
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

    pub fn chance_sampling<G, P>(
        &mut self,
        game: &G,
        n: &ActionNode<P, A>,
        player: P,
        pi: f64,
        po: f64,
    ) -> f64
    where
        G: Game<P, A>,
        P: Eq + Copy,
    {
        match n {
            ActionNode::NonTerminal(p, children) => {
                let info_set_str = game.info_set_str(*p, &self.history);
                let node = self
                    .nodes
                    .entry(info_set_str.clone())
                    .or_insert_with(|| Node::new(children.len()));
                let strategy = node.strategy().clone();

                let util: Vec<f64> = children
                    .iter()
                    .zip(strategy.iter())
                    .map(|((a, child), s)| {
                        self.history.push(*a);
                        let u = if *p == player {
                            self.chance_sampling(game, child, player, pi * s, po)
                        } else {
                            self.chance_sampling(game, child, player, pi, po * s)
                        };
                        self.history.pop();
                        u
                    })
                    .collect();
                let node_util = util.iter().zip(strategy.iter()).map(|(u, s)| u * s).sum();

                if let Some(node) = self.nodes.get_mut(&info_set_str) {
                    if *p == player {
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
            ActionNode::Terminal => game.utility(player, &self.history),
        }
    }

    pub fn external_sampling<G, P>(&mut self, game: &G, n: &ActionNode<P, A>, player: P) -> f64
    where
        G: Game<P, A>,
        P: Eq + Copy,
    {
        match n {
            ActionNode::NonTerminal(p, children) => {
                let info_set_str = game.info_set_str(*p, &self.history);
                if *p == player {
                    let util: Vec<f64> = children
                        .iter()
                        .map(|(a, child)| {
                            self.history.push(*a);
                            let u = self.external_sampling(game, child, player);
                            self.history.pop();
                            u
                        })
                        .collect();
                    let node = self
                        .nodes
                        .entry(info_set_str.clone())
                        .or_insert_with(|| Node::new(children.len()));
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
                        .or_insert_with(|| Node::new(children.len()));

                    node.update_strategy();
                    node.update_strategy_sum(1.);
                    let s = node.get_random_action();
                    let accion = children.get(s).unwrap();

                    self.history.push(accion.0);
                    let util = self.external_sampling(game, &accion.1, player);
                    self.history.pop();
                    util
                }
            }
            ActionNode::Terminal => game.utility(player, &self.history),
        }
    }
}

impl<A> Default for Cfr<A>
where
    A: Eq + Copy,
{
    fn default() -> Self {
        Self::new()
    }
}
