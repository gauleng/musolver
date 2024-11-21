use std::{cell::RefCell, rc::Rc};

use crate::mus::{Accion, PartidaMus};

use super::MusAction;

pub trait Kibitzer {
    fn record(&mut self, partida_mus: &PartidaMus, action: MusAction);
}

/// Kibitzer that records the actions played in a game. This kibitzer allows to share the game
/// history through its history() method.
pub struct ActionRecorder {
    history: Rc<RefCell<Vec<Accion>>>,
}

impl ActionRecorder {
    pub fn new() -> Self {
        Self {
            history: Rc::new(RefCell::new(vec![])),
        }
    }

    pub fn history(&self) -> Rc<RefCell<Vec<Accion>>> {
        self.history.clone()
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
            MusAction::GameStart(_) | MusAction::LanceStart(_) => self.history.borrow_mut().clear(),
            MusAction::PlayerAction(_, accion) => self.history.borrow_mut().push(*accion),
            _ => {}
        }
    }
}
