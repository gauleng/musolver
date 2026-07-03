use std::collections::VecDeque;

use crate::mus::Carta;
use arrayvec::ArrayVec;
use rand::seq::SliceRandom;
use rand::thread_rng;

use super::Mano;

/// Baraja española de cartas.
#[derive(Clone, Debug)]
pub struct Baraja(VecDeque<Carta>, usize);

impl Baraja {
    pub const FREC_BARAJA_MUS: [(Carta, u8); 8] = [
        (Carta::Rey, 8),
        (Carta::Caballo, 4),
        (Carta::Sota, 4),
        (Carta::Siete, 4),
        (Carta::Seis, 4),
        (Carta::Cinco, 4),
        (Carta::Cuatro, 4),
        (Carta::As, 8),
    ];

    /// Devuelve una nueva baraja vacía.
    pub fn new() -> Self {
        Baraja(VecDeque::with_capacity(40), 0)
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
        b.1 = 40;
        b
    }

    /// Genera cuatro manos a partir de las primeras dieciseis cartas de la baraja en el momento de
    /// la llamada a la función. Esta funcion no baraja las cartas y tampoco las elimina de la
    /// baraja.
    pub fn repartir_manos<const N: usize>(&mut self) -> [Mano; N] {
        let mut c = self.0.drain(0..4 * N);
        self.1 -= 16;
        core::array::from_fn(|_| {
            let mut m = ArrayVec::<Carta, 4>::new();
            for _ in 0..4 {
                m.push(c.next().unwrap());
            }
            Mano::from_arrayvec(m)
        })
    }

    /// Inserta una carta en la baraja.
    pub fn insertar(&mut self, c: Carta) {
        self.0.push_back(c);
        self.1 += 1;
    }

    /// Baraja las cartas. Utiliza el algoritmo shuffle del crate rand.
    pub fn barajar(&mut self) {
        self.0.make_contiguous().shuffle(&mut thread_rng());
    }

    /// Elimina una carta de la baraja y la devuelve. En caso de que sea una baraja vacía devuelve
    /// None.
    pub fn repartir(&mut self) -> Option<Carta> {
        self.1 -= 1;
        self.0.remove(0)
    }

    /// Devuelve un slice de las primeras n cartas de la baraja.
    pub fn primeras_n_cartas(&mut self, n: usize) -> &[Carta] {
        &self.0.make_contiguous()[0..n]
    }

    pub fn descartar(&mut self, mano: &mut Mano, descartes: [bool; 4]) {
        let mut num_descartes = 0;
        self.0
            .extend(mano.iter().enumerate().filter_map(|(idx, carta)| {
                if descartes[idx] {
                    num_descartes += 1;
                    Some(carta)
                } else {
                    None
                }
            }));
        if num_descartes > self.1 {
            self.0.make_contiguous()[self.1..].shuffle(&mut thread_rng());
            self.1 = self.0.len();
        }
        mano.reemplazar(descartes, self.0.drain(0..num_descartes));
        self.1 -= num_descartes;
    }
}

impl Default for Baraja {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_descartar() {
        let mut baraja = Baraja::new();
        let mut mano = Mano::new([Carta::As, Carta::As, Carta::As, Carta::Tres]);
        baraja.descartar(&mut mano, [true, true, true, true]);
        assert_eq!(mano.to_string(), "3111");

        baraja.insertar(Carta::Caballo);
        baraja.descartar(&mut mano, [false, false, true, true]);
        assert_eq!(mano.to_string(), "3C11");
    }
}
