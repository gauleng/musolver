use crate::mus::Carta;
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Clone, Debug)]
pub struct Baraja(Vec<Carta>);

impl Baraja {
    pub fn new() -> Self {
        Baraja(Vec::with_capacity(40))
    }

    pub fn baraja_mus() -> Baraja {
        let mut b = Baraja::new();
        for _ in 0..8 {
            b.insertar(Carta::As);
            b.insertar(Carta::Rey);
        }
        for _ in 0..4 {
            b.insertar(Carta::Caballo);
            b.insertar(Carta::Sota);
            b.insertar(Carta::Siete);
            b.insertar(Carta::Seis);
            b.insertar(Carta::Cinco);
            b.insertar(Carta::Cuatro);
        }
        b.barajar();
        b
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

    pub fn primeras_n_cartas(&self, n: usize) -> &[Carta] {
        &self.0[0..n]
    }
}

impl Default for Baraja {
    fn default() -> Self {
        Self::new()
    }
}
