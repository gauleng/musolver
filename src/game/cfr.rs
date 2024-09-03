use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use std::collections::HashMap;

use crate::mus::{self, Accion, Apuesta, Baraja, Carta, EstadoLance, Lance, Mano};

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
pub struct PartidaLance {
    manos: Vec<Mano>,
    lance: Lance,
    tantos: [u8; 2],
}

impl PartidaLance {
    pub fn new_random(lance: Lance, tantos: [u8; 2]) -> Self {
        let mut manos;
        loop {
            let b = Self::crear_baraja();
            manos = Self::repartir_manos(b);
            if lance.se_juega(&manos) {
                break;
            }
        }
        Self {
            manos,
            lance,
            tantos,
        }
    }
    fn crear_baraja() -> Baraja {
        let mut b = Baraja::new();
        for _ in 0..8 {
            b.insertar(mus::Carta::As);
            b.insertar(mus::Carta::Rey);
        }
        for _ in 0..4 {
            b.insertar(mus::Carta::Caballo);
            b.insertar(mus::Carta::Sota);
            b.insertar(mus::Carta::Siete);
            b.insertar(mus::Carta::Seis);
            b.insertar(mus::Carta::Cinco);
            b.insertar(mus::Carta::Cuatro);
        }
        b.barajar();
        b
    }

    fn repartir_manos(mut b: Baraja) -> Vec<Mano> {
        let mut manos = Vec::with_capacity(4);
        for _ in 0..4 {
            let mut m = Vec::<Carta>::with_capacity(4);
            for _ in 0..4 {
                m.push(b.repartir().unwrap());
            }
            manos.push(Mano::new(m));
        }
        manos
    }
}

#[derive(Debug)]
pub struct Cfr {
    history: Vec<Accion>,
    partida_lance: PartidaLance,
    nodos: HashMap<String, Node>,
}

impl Cfr {
    fn info_set_str(
        &self,
        player: usize,
        mano1: &Mano,
        mano2: &Mano,
        history: &[Accion],
    ) -> String {
        let mut output = String::with_capacity(11 + history.len() + 1);
        output.push(if player == 0 { '0' } else { '1' });
        output.push(',');
        output.push_str(&mano1.to_string());
        output.push(',');
        output.push_str(&mano2.to_string());
        output.push(',');
        for i in history.iter() {
            output.push_str(&i.to_string());
        }
        output
    }

    // fn info_set_str_one_hand(&self, player: usize, mano1: &Mano, history: &[Accion]) -> String {
    //     let mut output = String::with_capacity(11 + history.len() + 1);
    //     output.push(if player == 0 { '0' } else { '1' });
    //     output.push(',');
    //     output.push_str(&mano1.to_string());
    //     output.push(',');
    //     for i in history.iter() {
    //         output.push_str(&i.to_string());
    //     }
    //     output
    // }

    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            partida_lance: PartidaLance::new_random(Lance::Grande, [0, 0]),
            nodos: HashMap::new(),
        }
    }

    pub fn set_partida_lance(&mut self, h: PartidaLance) {
        self.partida_lance = h;
    }

    pub fn nodes(&self) -> &HashMap<String, Node> {
        &self.nodos
    }

    pub fn cfr(&mut self, n: &ActionNode<usize, Accion>, player: usize) -> f32 {
        match n {
            ActionNode::NonTerminal(p, children) => {
                let info_set_str = self.info_set_str(
                    *p,
                    &self.partida_lance.manos[*p],
                    &self.partida_lance.manos[*p + 2],
                    &self.history,
                );
                // let info_set_str =
                // self.info_set_str_one_hand(*p, &self.partida_lance.manos[*p], &self.history);
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
                let mut estado_lance =
                    EstadoLance::new(self.partida_lance.lance.apuesta_minima(), 40, 0);
                self.history.iter().for_each(|&a| {
                    let _ = estado_lance.actuar(a);
                });
                estado_lance.resolver_lance(&self.partida_lance.manos, &self.partida_lance.lance);
                let mut tantos: [u8; 2] = self.partida_lance.tantos;

                let ganador = estado_lance.ganador().unwrap();
                let apuesta = estado_lance.tantos_apostados();
                match apuesta {
                    Apuesta::Tantos(t) => tantos[ganador] += t,
                    Apuesta::Ordago => tantos[ganador] = 40,
                }
                if tantos[ganador] < 40 {
                    tantos[ganador] += self.partida_lance.lance.bonus();
                }

                // if tantos[ganador] < 40 {
                //     tantos[ganador] += Lance::Pares.tantos_mano(&self.manos[ganador]) as i8;
                // }
                let payoff = [tantos[0] - tantos[1], tantos[1] - tantos[0]];
                // println!(
                //     "Tantos para el jugador {}  con acciones {:?}: {}",
                //     player, self.history, tantos[player]
                // );
                payoff[player] as f32
            }
        }
    }
}
