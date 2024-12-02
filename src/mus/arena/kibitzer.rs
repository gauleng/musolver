use std::sync::{Arc, Mutex};

use crate::mus::{Accion, PartidaMus};

use super::MusAction;

pub trait Kibitzer {
    fn record(&mut self, partida_mus: &PartidaMus, action: MusAction);
}

/// Kibitzer that records the actions played in a game. This kibitzer allows to share the game
/// history through its history() method.
pub struct ActionRecorder {
    history: Arc<Mutex<Vec<Accion>>>,
}

impl ActionRecorder {
    pub fn new() -> Self {
        Self {
            history: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn history(&self) -> Arc<Mutex<Vec<Accion>>> {
        Arc::clone(&self.history)
    }
}

impl Default for ActionRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Kibitzer for ActionRecorder {
    fn record(&mut self, _partida_mus: &PartidaMus, action: MusAction) {
        match &action {
            MusAction::GameStart(_) | MusAction::LanceStart(_) => {
                self.history.lock().unwrap().clear()
            }
            MusAction::PlayerAction(_, accion) => self.history.lock().unwrap().push(*accion),
            _ => {}
        }
    }
}
