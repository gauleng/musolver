use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use std::collections::HashMap;

use crate::mus::{Accion, EstadoLance, Lance, Mano};

use super::ActionNode;

#[derive(Debug, Clone)]
pub struct Node {
    regret_sum: Vec<f32>,
    strategy: Vec<f32>,
    strategy_sum: Vec<f32>,
}

impl Node {
    fn new(num_actions: usize) -> Self {
        Self {
            regret_sum: vec![0.; num_actions],
            strategy: vec![0.; num_actions],
            strategy_sum: vec![0.; num_actions],
        }
    }

    pub fn get_strategy(&mut self) -> &Vec<f32> {
        for i in 0..self.strategy.len() {
            if self.regret_sum[i] > 0. {
                self.strategy[i] = self.regret_sum[i];
            } else {
                self.strategy[i] = 0.;
            }
        }
        let normalizing_sum: f32 = self.strategy.iter().sum();
        for i in 0..self.strategy.len() {
            if normalizing_sum > 0. {
                self.strategy[i] /= normalizing_sum;
            } else {
                self.strategy[i] = 1. / self.strategy.len() as f32;
            }
        }
        &self.strategy
    }
    pub fn get_average_strategy(&self) -> Vec<f32> {
        let mut avg_strategy = vec![0.; self.strategy.len()];
        let normalizing_sum: f32 = self.strategy_sum.iter().sum();
        for i in 0..self.strategy.len() {
            if normalizing_sum > 0. {
                avg_strategy[i] = self.strategy_sum[i] / normalizing_sum;
            } else {
                avg_strategy[i] = 1. / self.strategy.len() as f32;
            }
        }
        avg_strategy
    }

    pub fn get_random_action(&mut self) -> usize {
        let s = self.get_strategy();
        let dist = WeightedIndex::new(s).unwrap();
        for i in 0..self.strategy.len() {
            self.strategy_sum[i] += self.strategy[i];
        }
        dist.sample(&mut rand::thread_rng())
    }
}

#[derive(Debug)]
pub struct Cfr {
    history: Vec<Accion>,
    manos: Vec<Mano>,
    nodos: HashMap<String, Node>,
}

impl Cfr {
    // fn info_set_str(
    //     &self,
    //     player: usize,
    //     mano1: &Mano,
    //     mano2: &Mano,
    //     history: &[Accion],
    // ) -> String {
    //     let mut output = String::with_capacity(11 + history.len() + 1);
    //     output.push(if player == 0 { '0' } else { '1' });
    //     output.push(',');
    //     output.push_str(&mano1.to_string());
    //     output.push(',');
    //     output.push_str(&mano2.to_string());
    //     output.push(',');
    //     for i in history.iter() {
    //         output.push_str(&i.to_string());
    //     }
    //     output
    // }

    fn info_set_str_one_hand(&self, player: usize, mano1: &Mano, history: &[Accion]) -> String {
        let mut output = String::with_capacity(11 + history.len() + 1);
        output.push(if player == 0 { '0' } else { '1' });
        output.push(',');
        output.push_str(&mano1.to_string());
        output.push(',');
        for i in history.iter() {
            output.push_str(&i.to_string());
        }
        output
    }

    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            manos: vec![],
            nodos: HashMap::new(),
        }
    }

    pub fn set_hands(&mut self, h: Vec<Mano>) {
        self.manos = h;
    }

    pub fn nodes(&self) -> &HashMap<String, Node> {
        &self.nodos
    }

    pub fn cfr(&mut self, n: &ActionNode<usize, Accion>, player: usize) -> f32 {
        match n {
            ActionNode::NonTerminal(p, children) => {
                // let info_set_str =
                //     self.info_set_str(*p, &self.manos[*p], &self.manos[*p + 2], &self.history);
                let info_set_str = self.info_set_str_one_hand(*p, &self.manos[*p], &self.history);
                self.nodos
                    .entry(info_set_str.clone())
                    .or_insert(Node::new(children.len()));
                if *p == player {
                    let mut util = vec![0.; children.len()];
                    for (i, (a, child)) in children.iter().enumerate() {
                        self.history.push(*a);
                        util[i] = self.cfr(child, player);
                        self.history.pop();
                    }
                    let nodo = self.nodos.get_mut(&info_set_str).unwrap();
                    let strategy = nodo.get_strategy();
                    let mut node_util = 0.;

                    util.iter().enumerate().for_each(|(i, u)| {
                        node_util += strategy[i] * u;
                    });
                    util.iter().enumerate().for_each(|(i, u)| {
                        let regret = u - node_util;
                        nodo.regret_sum[i] += regret;
                    });
                    node_util
                } else {
                    let s = self
                        .nodos
                        .get_mut(&info_set_str)
                        .unwrap()
                        .get_random_action();
                    let accion = children.get(s).unwrap();

                    self.history.push(accion.0);
                    let util = self.cfr(&accion.1, player);
                    self.history.pop();
                    util
                }
            }
            ActionNode::Terminal => {
                let mut l = EstadoLance::new(1, 40, 0);
                self.history.iter().for_each(|&a| {
                    let _ = l.actuar(a);
                });
                l.resolver_lance(&self.manos, &Lance::Grande);
                let mut tantos: [i8; 2] = [0, 0];

                let ganador = l.ganador().unwrap();
                tantos[ganador] = l.tantos_apostados() as i8;
                // if tantos[ganador] < 40 {
                //     tantos[ganador] += Lance::Pares.tantos_mano(&self.manos[ganador]) as i8;
                // }
                tantos[1 - ganador] = -tantos[ganador];
                // println!(
                //     "Tantos para el jugador {}  con acciones {:?}: {}",
                //     player, self.history, tantos[player]
                // );
                tantos[player] as f32
            }
        }
    }
}
