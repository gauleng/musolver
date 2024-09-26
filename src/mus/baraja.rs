use crate::mus::Carta;
use rand::seq::SliceRandom;
use rand::thread_rng;

use super::Mano;

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

    pub fn repartir_manos(&self) -> [Mano; 4] {
        let mut c = self.primeras_n_cartas(16).iter();
        core::array::from_fn(|_| {
            let mut m = Vec::<Carta>::with_capacity(4);
            for _ in 0..4 {
                m.push(*c.next().unwrap());
            }
            Mano::new(m)
        })
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
