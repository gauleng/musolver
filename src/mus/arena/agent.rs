use std::{cell::RefCell, io, rc::Rc};

use rand::{distributions::WeightedIndex, prelude::Distribution, Rng};

use crate::{
    mus::{Accion, PartidaMus},
    solver::{LanceGameDosManos, Strategy},
    ActionNode, Game, Node,
};

pub trait Agent {
    fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion;
}

#[derive(Debug, Clone)]
pub struct AgenteCli {
    history: Rc<RefCell<Vec<Accion>>>,
    action_tree: ActionNode<usize, Accion>,
}

impl AgenteCli {
    pub fn new(action_tree: ActionNode<usize, Accion>, history: Rc<RefCell<Vec<Accion>>>) -> Self {
        Self {
            history,
            action_tree,
        }
    }
}

impl Agent for AgenteCli {
    fn actuar(&mut self, _partida_mus: &PartidaMus) -> Accion {
        println!("Elija una acción:");
        let node = self.action_tree.search_action_node(&self.history.borrow());
        if let ActionNode::NonTerminal(_, next_actions) = node {
            let acciones: Vec<Accion> = next_actions.iter().map(|c| c.0).collect();
            acciones
                .iter()
                .enumerate()
                .for_each(|(i, a)| println!("{i}: {:?}", a));
            let mut input = String::new();
            loop {
                io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");
                let num = input.trim().parse::<usize>();
                match num {
                    Ok(n) => {
                        if n < acciones.len() {
                            return acciones[n];
                        } else {
                            println!("Opción no válida.");
                        }
                    }
                    Err(_) => {
                        println!("Opción no válida.");
                        input.clear();
                    }
                }
            }
        }
        Accion::Paso
    }
}

#[derive(Debug, Clone)]
pub struct AgenteAleatorio {
    history: Rc<RefCell<Vec<Accion>>>,
    action_tree: ActionNode<usize, Accion>,
}

impl AgenteAleatorio {
    pub fn new(action_tree: ActionNode<usize, Accion>, history: Rc<RefCell<Vec<Accion>>>) -> Self {
        Self {
            history,
            action_tree,
        }
    }
}

impl Agent for AgenteAleatorio {
    fn actuar(&mut self, _partida_mus: &PartidaMus) -> Accion {
        let next_actions = self
            .action_tree
            .search_action_node(&self.history.borrow())
            .children();
        match next_actions {
            None => {
                println!(
                    "ERROR: La lista de acciones no está en el árbol. {:?}. Se pasa por defecto.",
                    self.history.borrow()
                );
                Accion::Paso
            }
            Some(c) => {
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0..c.len());
                c[idx].0
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgenteMusolver {
    strategy: Strategy,
    history: Rc<RefCell<Vec<Accion>>>,
}

impl AgenteMusolver {
    pub fn new(strategy: Strategy, history: Rc<RefCell<Vec<Accion>>>) -> Self {
        Self { strategy, history }
    }

    fn accion_aleatoria(&mut self, partida_mus: &PartidaMus, acciones: Vec<Accion>) -> Accion {
        let lance = partida_mus.lance_actual().unwrap();
        let turno_inicial = lance.turno_inicial(partida_mus.manos());
        let mut turno = partida_mus.turno().unwrap();
        if turno_inicial == 1 {
            turno = 1 - turno;
        }
        let info_set = LanceGameDosManos::from_partida_mus(
            partida_mus,
            self.strategy.strategy_config.game_config.abstract_game,
        )
        .unwrap()
        .info_set_str(turno);
        let probabilities = match self.strategy.nodes.get(&info_set) {
            None => {
                println!("ERROR: InfoSet no encontrado: {info_set}");
                &Node::new(acciones.len()).strategy().clone()
            }
            Some(n) => n,
        };

        let dist = WeightedIndex::new(probabilities).unwrap();
        let idx = dist.sample(&mut rand::thread_rng());
        acciones[idx]
    }
}

impl Agent for AgenteMusolver {
    fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion {
        let next_actions = self
            .strategy
            .strategy_config
            .trainer_config
            .action_tree
            .search_action_node(&self.history.borrow())
            .children();
        match next_actions {
            None => {
                println!(
                    "ERROR: La lista de acciones no está en el árbol. {:?}. Se pasa por defecto.",
                    self.history.borrow()
                );
                Accion::Paso
            }
            Some(c) => {
                let acciones: Vec<Accion> = c.iter().map(|a| a.0).collect();
                self.accion_aleatoria(partida_mus, acciones)
            }
        }
    }
}
