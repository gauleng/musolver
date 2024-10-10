use crate::mus::Carta;
use rand::seq::SliceRandom;
use rand::thread_rng;

use super::Mano;

/// Baraja española de cartas.
#[derive(Clone, Debug)]
pub struct Baraja(Vec<Carta>);

impl Baraja {
    pub const FREC_BARAJA_MUS: [(Carta, u8); 8] = [
        (Carta::As, 8),
        (Carta::Cuatro, 4),
        (Carta::Cinco, 4),
        (Carta::Seis, 4),
        (Carta::Siete, 4),
        (Carta::Sota, 4),
        (Carta::Caballo, 4),
        (Carta::Rey, 8),
    ];

    /// Devuelve una nueva baraja vacía.
    pub fn new() -> Self {
        Baraja(Vec::with_capacity(40))
    }

    /// Devuelve una baraj de mus. Incluye ocho ases y ocho reyes, y no incluye ni doses ni treses.
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

    /// Genera cuatro manos a partir de las primeras dieciseis cartas de la baraja en el momento de
    /// la llamada a la función. Esta funcion no baraja las cartas y tampoco las elimina de la
    /// baraja.
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

    /// Inserta una carta en la baraja.
    pub fn insertar(&mut self, c: Carta) {
        self.0.push(c);
    }

    /// Baraja las cartas. Utiliza el algoritmo shuffle del crate rand.
    pub fn barajar(&mut self) {
        self.0.shuffle(&mut thread_rng());
    }

    /// Elimina una carta de la baraja y la devuelve. En caso de que sea una baraja vacía devuelve
    /// None.
    pub fn repartir(&mut self) -> Option<Carta> {
        self.0.pop()
    }

    /// Devuelve un slice de las primeras n cartas de la baraja.
    pub fn primeras_n_cartas(&self, n: usize) -> &[Carta] {
        &self.0[0..n]
    }
}

impl Default for Baraja {
    fn default() -> Self {
        Self::new()
    }
}
