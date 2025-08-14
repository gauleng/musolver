use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rand::{distributions::WeightedIndex, prelude::Distribution, Rng};

use crate::{
    mus::{Accion, PartidaMus},
    solver::{LanceGame, Strategy},
    Game, NodeType,
};

#[async_trait]
pub trait Agent {
    async fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion;
}

#[derive(Debug, Clone)]
pub struct AgenteAleatorio {
    history: Arc<Mutex<Vec<Accion>>>,
}

impl AgenteAleatorio {
    pub fn new(history: Arc<Mutex<Vec<Accion>>>) -> Self {
        Self { history }
    }
}

#[async_trait]
impl Agent for AgenteAleatorio {
    async fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion {
        let mut lance_game = LanceGame::from_partida_mus(partida_mus, true).unwrap();
        let history = self.history.lock().unwrap().clone();
        for action in &history {
            lance_game.act(*action);
        }
        let actions = lance_game.actions();
        if actions.is_empty() {
            println!(
                "ERROR: La lista de acciones no está en el árbol. {history:?}. Se pasa por defecto."
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
    history: Arc<Mutex<Vec<Accion>>>,
}

impl AgenteMusolver {
    pub fn new(strategy: Strategy<LanceGame>, history: Arc<Mutex<Vec<Accion>>>) -> Self {
        Self { strategy, history }
    }

    fn accion_aleatoria(actions: &[Accion], probabilities: &[f64]) -> Accion {
        let dist = WeightedIndex::new(probabilities).unwrap();
        let idx = dist.sample(&mut rand::thread_rng());
        actions[idx]
    }
}

#[async_trait]
impl Agent for AgenteMusolver {
    async fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion {
        let mut lance_game = LanceGame::from_partida_mus(
            partida_mus,
            self.strategy.strategy_config.game_config.abstract_game,
        )
        .unwrap();
        let history = self.history.lock().unwrap().clone();
        for action in &history {
            lance_game.act(*action);
        }
        let current_player = match lance_game.current_player() {
            NodeType::Player(current_player) => current_player,
            _ => 0,
        };
        let info_set_str = lance_game.info_set_str(current_player);
        let action_probability = self.strategy.nodes.get(&info_set_str);
        if let Some((actions, probabilities)) = action_probability {
            Self::accion_aleatoria(actions, probabilities)
        } else {
            println!(
                "ERROR: La lista de acciones no está en el árbol. {history:?}. Se pasa por defecto."
            );
            Accion::Paso
        }
    }
}
