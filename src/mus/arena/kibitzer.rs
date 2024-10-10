use std::{cell::RefCell, rc::Rc};

use crate::mus::{Accion, Juego, Lance, Mano, PartidaMus};

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

pub struct KibitzerCli {
    manos: Vec<Mano>,
    marcador: [usize; 2],
    cli_player: usize,
    pareja_mano: usize,
    lance_actual: Option<Lance>,
}

impl KibitzerCli {
    pub fn new(cli_player: usize) -> Self {
        Self {
            manos: vec![],
            marcador: [0, 0],
            cli_player,
            pareja_mano: 0,
            lance_actual: None,
        }
    }

    fn hand_str(lance: &Lance, m: &Mano, hidden: bool) -> String {
        let hay_jugada = match lance {
            Lance::Grande | Lance::Chica | Lance::Punto => false,
            Lance::Pares => m.pares().is_some(),
            Lance::Juego => m.juego().is_some(),
        };
        let ayuda_valor = match lance {
            Lance::Juego => m
                .juego()
                .map(|j| match j {
                    Juego::Resto(v) => format!("({v})"),
                    Juego::Treintaydos => "(32)".to_string(),
                    Juego::Treintayuna => "(31)".to_string(),
                })
                .unwrap_or_default(),
            Lance::Punto => format!("({})", m.valor_puntos()),
            _ => "".to_string(),
        };
        let suffix = if hay_jugada {
            "*".to_owned()
        } else {
            "".to_owned()
        };
        if hidden {
            format!("XXXX {suffix}")
        } else {
            format!("{m} {ayuda_valor} {suffix}")
        }
    }
}

impl Kibitzer for KibitzerCli {
    fn record(&mut self, partida_mus: &PartidaMus, action: MusAction) {
        match &action {
            MusAction::GameStart(p) => {
                self.lance_actual = None;
                self.pareja_mano = *p;
                self.manos.clear();
                println!();
                println!();
                println!("🥊🥊🥊 Game starts! Fight! 🥊🥊🥊");
                println!("Marcador: {}-{}", self.marcador[0], self.marcador[1]);
                println!();
                println!();
            }
            MusAction::DealHand(p, m) => {
                let lance = partida_mus.lance_actual().unwrap();
                let hand_str = if self.pareja_mano == self.cli_player && p % 2 == 0
                    || self.pareja_mano != self.cli_player && p % 2 == 1
                {
                    KibitzerCli::hand_str(&lance, m, false)
                } else {
                    KibitzerCli::hand_str(&lance, m, true)
                };
                println!("Mano jugador {p}: {hand_str}");
                self.manos.push(m.clone());
            }
            MusAction::PlayerAction(p, accion) => {
                if *p != self.cli_player {
                    println!("❗❗❗Pareja {p} ha actuado: {:?}", accion);
                }
            }
            MusAction::Payoff(p, t) => {
                if *t > 0 {
                    let pareja = if *p == self.pareja_mano { 0 } else { 1 };
                    if *p == self.cli_player {
                        println!();
                        println!("¡¡¡¡HAS GANADO {t} tantos!!!! 🚀🚀🚀");
                        println!();
                        println!(
                            "Manos del rival: {} {}",
                            KibitzerCli::hand_str(
                                &self.lance_actual.unwrap(),
                                &self.manos[1 - pareja],
                                false
                            ),
                            KibitzerCli::hand_str(
                                &self.lance_actual.unwrap(),
                                &self.manos[3 - pareja],
                                false
                            ),
                        );
                    } else {
                        println!(
                            "Pareja {p} ha ganado {t} tantos con manos: {} {}",
                            KibitzerCli::hand_str(
                                &self.lance_actual.unwrap(),
                                &self.manos[pareja],
                                false
                            ),
                            KibitzerCli::hand_str(
                                &self.lance_actual.unwrap(),
                                &self.manos[pareja + 2],
                                false
                            ),
                        );
                    }
                }
                self.marcador[*p] += *t as usize;
            }
            MusAction::LanceStart(lance) => self.lance_actual = Some(*lance),
        }
    }
}
