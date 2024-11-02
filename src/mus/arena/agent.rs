use std::{cell::RefCell, io, rc::Rc};

use rand::{distributions::WeightedIndex, prelude::Distribution, Rng};

use crate::{
    mus::{Accion, PartidaMus, Turno},
    solver::{LanceGame, Strategy},
    Game, Node,
};

pub trait Agent {
    fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion;
}

#[derive(Debug, Clone)]
pub struct AgenteCli {
    history: Rc<RefCell<Vec<Accion>>>,
}

impl AgenteCli {
    pub fn new(history: Rc<RefCell<Vec<Accion>>>) -> Self {
        Self { history }
    }
}

impl Agent for AgenteCli {
    fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion {
        let mut lance_game = LanceGame::from_partida_mus(partida_mus, true).unwrap();
        for action in self.history.borrow().iter() {
            lance_game.act(*action);
        }
        println!("Elija una acción:");
        if let Some(next_actions) = lance_game.actions() {
            next_actions
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
                        if n < next_actions.len() {
                            return next_actions[n];
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
}

impl AgenteAleatorio {
    pub fn new(history: Rc<RefCell<Vec<Accion>>>) -> Self {
        Self { history }
    }
}

impl Agent for AgenteAleatorio {
    fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion {
        let mut lance_game = LanceGame::from_partida_mus(partida_mus, true).unwrap();
        for action in self.history.borrow().iter() {
            lance_game.act(*action);
        }
        match lance_game.actions() {
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
                c[idx]
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
        let turno = partida_mus.turno().unwrap();
        let player_id = match turno {
            Turno::Jugador(player_id) => player_id,
            Turno::Pareja(player_id) => player_id,
        } as usize;
        let info_set = LanceGame::from_partida_mus(
            partida_mus,
            self.strategy.strategy_config.game_config.abstract_game,
        )
        .unwrap()
        .info_set_str(player_id);
        let probabilities = match self.strategy.nodes.get(&info_set) {
            None => {
                println!("ERROR: InfoSet no encontrado: {info_set}");
                &Node::new(acciones.clone()).strategy().to_owned()
            }
            Some(n) => &n.iter().map(|(_, p)| *p).collect(),
        };

        let dist = WeightedIndex::new(probabilities).unwrap();
        let idx = dist.sample(&mut rand::thread_rng());
        acciones[idx]
    }
}

impl Agent for AgenteMusolver {
    fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion {
        let mut lance_game = LanceGame::from_partida_mus(
            partida_mus,
            self.strategy.strategy_config.game_config.abstract_game,
        )
        .unwrap();
        for action in self.history.borrow().iter() {
            lance_game.act(*action);
        }
        let next_actions = lance_game.actions();
        match next_actions {
            None => {
                println!(
                    "ERROR: La lista de acciones no está en el árbol. {:?}. Se pasa por defecto.",
                    self.history.borrow()
                );
                Accion::Paso
            }
            Some(c) => self.accion_aleatoria(partida_mus, c),
        }
    }
}
