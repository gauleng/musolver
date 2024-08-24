use mus::{Accion, Baraja, Carta, EstadoLance, Lance, Mano};
use musolver::*;
use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use std::collections::HashMap;

#[derive(Debug)]
struct Node {
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
        dist.sample(&mut rand::thread_rng())
    }

    pub fn update_strategy(&mut self) {
        for i in 0..self.strategy.len() {
            self.strategy_sum[i] += self.strategy[i];
        }
    }
}

#[derive(Debug)]
struct Cfr {
    history: Vec<Accion>,
    manos: Vec<Mano>,
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
        output.push('-');
        output.push_str(&mano1.to_string());
        output.push(',');
        output.push_str(&mano2.to_string());
        for i in history.iter() {
            output.push_str(&i.to_string());
        }
        output
    }

    fn cfr(&mut self, n: &ActionNode2<usize, Accion>, player: usize) -> f32 {
        match n {
            ActionNode2::NonTerminal(p, children) => {
                let info_set_str =
                    self.info_set_str(*p, &self.manos[*p], &self.manos[*p + 2], &self.history);
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
                    // Elegir una acción aleatoria

                    let s = self
                        .nodos
                        .get_mut(&info_set_str)
                        .unwrap()
                        .get_random_action();
                    let accion = children.get(s).unwrap();

                    self.history.push(accion.0);
                    let util = self.cfr(&accion.1, player);
                    self.history.pop();
                    self.nodos.get_mut(&info_set_str).unwrap().update_strategy();
                    util
                }
            }
            ActionNode2::Terminal => {
                let mut l = EstadoLance::new(1, 40);
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

fn crear_baraja() -> Baraja {
    let mut b = Baraja::new();
    for _ in 0..8 {
        b.insertar(mus::Carta::As);
        b.insertar(mus::Carta::Rey);
    }
    for _ in 0..4 {
        // b.insertar(mus::Carta::Caballo);
        // b.insertar(mus::Carta::Sota);
        // b.insertar(mus::Carta::Siete);
        // b.insertar(mus::Carta::Seis);
        // b.insertar(mus::Carta::Cinco);
        // b.insertar(mus::Carta::Cuatro);
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

fn help() {
    println!("Use: musolver <num iterations>");
}

fn init_action_tree() -> ActionNode2<usize, Accion> {
    let mut n = ActionNode2::<usize, Accion>::new(0);
    let p1paso = n.add_non_terminal_action(Accion::Paso, 1).unwrap();
    p1paso.add_terminal_action(Accion::Paso);
    let p2paso_envido = p1paso
        .add_non_terminal_action(Accion::Envido(2), 0)
        .unwrap();
    p2paso_envido.add_terminal_action(Accion::Paso);
    p2paso_envido.add_terminal_action(Accion::Quiero);
    let p1paso_envido_ordago = p2paso_envido
        .add_non_terminal_action(Accion::Ordago, 1)
        .unwrap();
    p1paso_envido_ordago.add_terminal_action(Accion::Paso);
    p1paso_envido_ordago.add_terminal_action(Accion::Quiero);
    let p2paso_ordago = p1paso.add_non_terminal_action(Accion::Ordago, 0).unwrap();
    p2paso_ordago.add_terminal_action(Accion::Paso);
    p2paso_ordago.add_terminal_action(Accion::Quiero);
    let p1envido = n.add_non_terminal_action(Accion::Envido(2), 1).unwrap();
    p1envido.add_terminal_action(Accion::Paso);
    p1envido.add_terminal_action(Accion::Quiero);
    let p2envido_ordago = p1envido.add_non_terminal_action(Accion::Ordago, 0).unwrap();
    p2envido_ordago.add_terminal_action(Accion::Paso);
    p2envido_ordago.add_terminal_action(Accion::Quiero);
    let p1ordago = n.add_non_terminal_action(Accion::Ordago, 1).unwrap();
    p1ordago.add_terminal_action(Accion::Paso);
    p1ordago.add_terminal_action(Accion::Quiero);

    n
}

fn external_cfr(iter: usize) {
    use std::time::Instant;

    let mut c = Cfr {
        history: Vec::new(),
        manos: vec![],
        nodos: HashMap::new(),
    };
    let action_tree = init_action_tree();

    let now = Instant::now();
    let mut baraja = crear_baraja();
    for _ in 0..iter {
        baraja.barajar();
        let b = baraja.clone();
        let m = repartir_manos(b);
        c.manos = m;
        c.cfr(&action_tree, 0);
        c.cfr(&action_tree, 1);
        //println!("{:?}", c.nodos);
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
    let mut v: Vec<(String, Node)> = c.nodos.into_iter().collect();
    v.sort_by(|x, y| x.0.cmp(&y.0));
    for (k, n) in v {
        println!("{:?} => {:?}", k, n.get_average_strategy());
    }
}

fn main() {
    // let mut n = ActionNode::<usize, Accion>::new(0);
    // let p1 = n.add_non_terminal_action(Accion::Paso, 1).unwrap();
    // p1.add_terminal_action(Accion::Paso);
    // let p2 = p1.add_non_terminal_action(Accion::Envido(2), 0).unwrap();
    // p2.add_terminal_action(Accion::Quiero);
    // let p3 = n.add_non_terminal_action(Accion::Envido(2), 1).unwrap();
    // p3.add_terminal_action(Accion::Paso);
    // p3.add_terminal_action(Accion::Quiero);

    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        0 | 1 => help(),
        2 => {
            let num_iter: usize = match args[1].parse() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("Second argument is not an integer.");
                    help();
                    return;
                }
            };
            external_cfr(num_iter);
        }
        _ => {}
    }
}
