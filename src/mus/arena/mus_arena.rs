use crate::mus::{Accion, Baraja, Lance, Mano, PartidaMus};

use super::{Agent, Kibitzer};

/// Events generated by MusArena during a game.
#[derive(Clone, Debug)]
pub enum MusAction {
    /// Game starts.
    GameStart(usize),
    /// Cards are dealed to players. It contains the index of the player receiving its cards and
    /// the hand itself.
    DealHand(usize, Mano),
    /// A new lance starts.
    LanceStart(Lance),
    /// A player acts. It contains the index of the player acting and the corresponding action.
    PlayerAction(usize, Accion),
    /// The payoff received by each player at the end of the game. It contains the index of the
    /// player receiving the payoff and the payoff itself.
    Payoff(usize, u8),
}

/// Simulates a mus game or a particular lance.
pub struct MusArena {
    pub agents: Vec<Box<dyn Agent>>,
    pub kibitzers: Vec<Box<dyn Kibitzer>>,
    partida_mus: PartidaMus,
    lance: Option<Lance>,
    order: [usize; 2],
}

impl MusArena {
    pub fn new(lance: Option<Lance>) -> Self {
        MusArena {
            agents: vec![],
            kibitzers: vec![],
            partida_mus: MusArena::new_partida(lance),
            order: [0, 1],
            lance,
        }
    }

    fn new_partida(lance: Option<Lance>) -> PartidaMus {
        let mut baraja = Baraja::baraja_mus();
        match lance {
            None => {
                let manos = baraja.repartir_manos();
                PartidaMus::new(manos, [0, 0])
            }
            Some(lance) => loop {
                baraja.barajar();
                let manos = baraja.repartir_manos();
                let posible_partida_mus = PartidaMus::new_partida_lance(lance, manos, [0, 0]);
                if let Some(partida_mus) = posible_partida_mus {
                    return partida_mus;
                }
            },
        }
    }

    fn record_action(&mut self, a: MusAction) {
        self.kibitzers
            .iter_mut()
            .for_each(|k| k.record(&self.partida_mus, a.clone()));
    }

    pub fn start(&mut self) {
        self.partida_mus = MusArena::new_partida(self.lance);
        self.order.swap(0, 1);
        self.record_action(MusAction::GameStart(self.order[0]));
        let manos = self.partida_mus.manos().clone();
        for (i, m) in manos.iter().enumerate() {
            self.record_action(MusAction::DealHand(i, m.clone()));
        }
        let mut lance = self.partida_mus.lance_actual();
        self.record_action(MusAction::LanceStart(lance.unwrap()));
        while let Some(turno) = self.partida_mus.turno() {
            let accion = self.agents[self.order[turno]].actuar(&self.partida_mus);
            if self.partida_mus.actuar(accion).is_ok() {
                self.record_action(MusAction::PlayerAction(self.order[turno], accion));
                let nuevo_lance = self.partida_mus.lance_actual();
                if nuevo_lance != lance {
                    lance = nuevo_lance;
                    if let Some(l) = lance {
                        self.record_action(MusAction::LanceStart(l));
                    }
                }
            }
        }
        let tantos = *self.partida_mus.tantos();
        self.record_action(MusAction::Payoff(self.order[0], tantos[0]));
        self.record_action(MusAction::Payoff(self.order[1], tantos[1]));
    }
}