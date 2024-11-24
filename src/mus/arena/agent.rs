use std::{cell::RefCell, rc::Rc};

use rand::{distributions::WeightedIndex, prelude::Distribution, Rng};

use crate::{
    mus::{Accion, PartidaMus},
    solver::{LanceGame, Strategy},
    Game,
};

pub trait Agent {
    fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion;
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
        let actions = lance_game.actions();
        if actions.is_empty() {
            println!(
                "ERROR: La lista de acciones no está en el árbol. {:?}. Se pasa por defecto.",
                self.history.borrow()
            );
            return Accion::Paso;
        }
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..actions.len());
        actions[idx]
    }
}

#[derive(Debug, Clone)]
pub struct AgenteMusolver {
    strategy: Strategy<LanceGame>,
    history: Rc<RefCell<Vec<Accion>>>,
}

impl AgenteMusolver {
    pub fn new(strategy: Strategy<LanceGame>, history: Rc<RefCell<Vec<Accion>>>) -> Self {
        Self { strategy, history }
    }

    fn accion_aleatoria(actions: &[Accion], probabilities: &[f64]) -> Accion {
        let dist = WeightedIndex::new(probabilities).unwrap();
        let idx = dist.sample(&mut rand::thread_rng());
        actions[idx]
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
        let current_player = lance_game.current_player().unwrap();
        let action_probability = self
            .strategy
            .nodes
            .get(&lance_game.info_set_str(current_player));
        if let Some((actions, probabilities)) = action_probability {
            Self::accion_aleatoria(actions, probabilities)
        } else {
            println!(
                "ERROR: La lista de acciones no está en el árbol. {:?}. Se pasa por defecto.",
                self.history.borrow()
            );
            Accion::Paso
        }
    }
}
