use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rand::{Rng, distributions::WeightedIndex, prelude::Distribution};

use crate::{
    Game, NodeType,
    mus::{Accion, CuatroJugadores, DosJugadores, ModalidadMus, PartidaMus},
    solver::{LanceGame, Strategy},
};

#[async_trait]
pub trait Agent<T: ModalidadMus> {
    async fn actuar(&mut self, partida_mus: &PartidaMus<T>) -> Accion;
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
impl Agent<CuatroJugadores> for AgenteAleatorio {
    async fn actuar(&mut self, partida_mus: &PartidaMus<CuatroJugadores>) -> Accion {
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
    strategy: Strategy,
    initial_score: [u8; 2],
    history: Arc<Mutex<Vec<Accion>>>,
}

impl AgenteMusolver {
    pub fn new(strategy: Strategy, history: Arc<Mutex<Vec<Accion>>>) -> Self {
        Self {
            strategy,
            initial_score: [0, 0],
            history,
        }
    }

    fn accion_aleatoria(actions: &[Accion], probabilities: &[f64]) -> Accion {
        let dist = WeightedIndex::new(probabilities).unwrap();
        let idx = dist.sample(&mut rand::thread_rng());
        actions[idx]
    }
}

#[async_trait]
impl Agent<DosJugadores> for AgenteMusolver {
    async fn actuar(&mut self, partida_mus: &PartidaMus<DosJugadores>) -> Accion {
        let history = self.history.lock().unwrap().clone();
        if history.len() < 2 {
            self.initial_score = *partida_mus.tantos();
        }
        let action_probability =
            self.strategy
                .actions(partida_mus.manos(), self.initial_score, &history);
        if let Some((actions, probabilities)) = action_probability {
            Self::accion_aleatoria(&actions, &probabilities)
        } else {
            println!(
                "ERROR: La lista de acciones no está en el árbol. {history:?}. Se pasa por defecto."
            );
            Accion::Paso
        }
    }
}

#[async_trait]
impl Agent<CuatroJugadores> for AgenteMusolver {
    async fn actuar(&mut self, partida_mus: &PartidaMus<CuatroJugadores>) -> Accion {
        let history = self.history.lock().unwrap().clone();
        if history.len() < 2 {
            self.initial_score = *partida_mus.tantos();
        }
        let action_probability =
            self.strategy
                .actions(partida_mus.manos(), self.initial_score, &history);
        if let Some((actions, probabilities)) = action_probability {
            Self::accion_aleatoria(&actions, &probabilities)
        } else {
            println!(
                "ERROR: La lista de acciones no está en el árbol. {history:?}. Se pasa por defecto."
            );
            Accion::Paso
        }
    }
}
