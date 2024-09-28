use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use super::ActionNode;

#[derive(Debug, Clone)]
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
    fn utility(&self, player: P, history: &[A]) -> f64;
    fn info_set_str(&self, player: P, history: &[A]) -> String;
    fn new_random(&mut self);
}

#[derive(Debug, Clone)]
pub struct Cfr<A> {
    history: Vec<A>,
    nodos: HashMap<String, Node>,
}

impl<A> Cfr<A>
where
    A: Eq + Copy,
{
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            nodos: HashMap::new(),
        }
    }

    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(file);
        let mut s = Self::new();
        for result in rdr.records() {
            let record = result?;
            let info_set = format!(
                "{},{},{},{}",
                &record[0], &record[1], &record[2], &record[3]
            );
            let mut node = Node::new(record.len() - 4);
            for i in 4..record.len() {
                let num = record[i].parse::<f64>();
                if let Ok(v) = num {
                    node.strategy[i - 4] = v;
                }
            }
            s.nodos.insert(info_set, node);
        }
        Ok(s)
    }

    pub fn to_file(&self, path: &Path) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut wtr = csv::WriterBuilder::new()
            .flexible(true)
            .quote_style(csv::QuoteStyle::Never)
            .from_writer(&file);

        let mut v: Vec<(String, Node)> = self
            .nodes()
            .iter()
            .map(|(s, n)| (s.clone(), n.clone()))
            .collect();
        v.sort_by(|x, y| x.0.cmp(&y.0));
        for (k, n) in v {
            let mut probabilities = n
                .get_average_strategy()
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>();
            probabilities.insert(0, k);
            wtr.write_record(&probabilities)?;
        }
        wtr.flush()?;
        Ok(())
    }

    pub fn nodes(&self) -> &HashMap<String, Node> {
        &self.nodos
    }

    pub fn update_strategy(&mut self) {
        self.nodos.values_mut().for_each(|n| {
            n.update_strategy();
        });
    }

    pub fn chance_cfr<G, P>(
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
                self.nodos
                    .entry(info_set_str.clone())
                    .or_insert(Node::new(children.len()));
                let node = self.nodos.get(&info_set_str).unwrap();
                let strategy = node.strategy().clone();

                let util: Vec<f64> = children
                    .iter()
                    .zip(strategy.iter())
                    .map(|((a, child), s)| {
                        self.history.push(*a);
                        let u = if *p == player {
                            self.chance_cfr(game, child, player, pi * s, po)
                        } else {
                            self.chance_cfr(game, child, player, pi, po * s)
                        };
                        self.history.pop();
                        u
                    })
                    .collect();
                let node_util = util.iter().zip(strategy.iter()).map(|(u, s)| u * s).sum();

                if let Some(node) = self.nodos.get_mut(&info_set_str) {
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

    pub fn external_cfr<G, P>(&mut self, game: &G, n: &ActionNode<P, A>, player: P) -> f64
    where
        G: Game<P, A>,
        P: Eq + Copy,
    {
        match n {
            ActionNode::NonTerminal(p, children) => {
                let info_set_str = game.info_set_str(*p, &self.history);
                self.nodos
                    .entry(info_set_str.clone())
                    .or_insert(Node::new(children.len()));
                if *p == player {
                    let util: Vec<f64> = children
                        .iter()
                        .map(|(a, child)| {
                            self.history.push(*a);
                            let u = self.external_cfr(game, child, player);
                            self.history.pop();
                            u
                        })
                        .collect();
                    let nodo = self.nodos.get_mut(&info_set_str).unwrap();
                    let strategy = nodo.update_strategy();

                    let node_util = util.iter().zip(strategy.iter()).map(|(u, s)| u * s).sum();
                    nodo.regret_sum
                        .iter_mut()
                        .zip(util.iter())
                        .for_each(|(r, u)| *r += u - node_util);
                    node_util
                } else {
                    let node = self.nodos.get_mut(&info_set_str).unwrap();

                    node.update_strategy();
                    node.update_strategy_sum(1.);
                    let s = node.get_random_action();
                    let accion = children.get(s).unwrap();

                    self.history.push(accion.0);
                    let util = self.external_cfr(game, &accion.1, player);
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
