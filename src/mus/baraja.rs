use crate::mus::Carta;
use rand::seq::SliceRandom;
use rand::thread_rng;

pub struct Baraja(Vec<Carta>);

impl Baraja {
    pub fn new() -> Self {
        Baraja(vec![])
    }

    pub fn insertar(&mut self, c: Carta) {
        self.0.push(c);
    }

    pub fn barajar(&mut self) {
        self.0.shuffle(&mut thread_rng());
    }

    pub fn repartir(&mut self) -> Option<Carta> {
        self.0.pop()
    }
}
