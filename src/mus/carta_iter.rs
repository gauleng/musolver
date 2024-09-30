use std::ops::Range;

use itertools::{CombinationsWithReplacement, Itertools};

use super::Carta;

use rug::Integer;

pub struct CartaIter<'a> {
    cartas: &'a [Carta],
    iter: CombinationsWithReplacement<Range<usize>>,
}

/// Iterador de manos de cartas de mus.
impl<'a> CartaIter<'a> {
    /// Crea un nuevo iterador a partir de un slice de Cartas y el número de cartas que se desean
    /// tener en la mano. Este iterador asume que las cartas se pueden repetir. Por ejemplo, si
    /// tenemos:
    ///
    /// let cartas = [Carta::As, Carta::Cuatro, Carta::Rey]
    /// let iter = CartaIter::new(&cartas, 2);
    ///
    /// assert_eq!(iter.next().unwrap(), &[Carta::As, Carta::As]);
    /// assert_eq!(iter.next().unwrap(), &[Carta::Cuatro, Carta::As]);
    /// assert_eq!(iter.next().unwrap(), &[Carta::Rey, Carta::As]);
    /// assert_eq!(iter.next().unwrap(), &[Carta::Cuatro, Carta::Cuatro]);
    pub fn new(cartas: &'a [Carta], num_cartas: usize) -> Self {
        let iter: CombinationsWithReplacement<Range<usize>> =
            (0..cartas.len()).combinations_with_replacement(num_cartas);
        Self { cartas, iter }
    }
}

impl<'a> Iterator for CartaIter<'a> {
    type Item = Vec<Carta>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next();
        next.map(|indices| indices.iter().map(|idx| self.cartas[*idx]).collect())
    }
}

pub struct DistribucionCartaIter<'a> {
    cartas: &'a [(Carta, u8)],
    iter: CombinationsWithReplacement<Range<usize>>,
}

/// Iterador de manos de cartas de mus.
impl<'a> DistribucionCartaIter<'a> {
    /// Crea un nuevo iterador a partir de una distribución de cartas y el número de cartas que se
    /// desean tener ne lam ano. La distribución se indica con un vector de pares (Carta, u8),
    /// donde el entero indica el número de cartas disponibles de ese valor.
    pub fn new(cartas: &'a [(Carta, u8)], num_cartas: usize) -> Self {
        let iter: CombinationsWithReplacement<Range<usize>> =
            (0..cartas.len()).combinations_with_replacement(num_cartas);
        Self { cartas, iter }
    }
}

impl<'a> Iterator for DistribucionCartaIter<'a> {
    type Item = (Vec<Carta>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        'outer: loop {
            let next = self.iter.next();
            let indices = next?;
            let mut contadores = vec![0; self.cartas.len()];

            for idx in &indices {
                contadores[*idx] += 1;
                if contadores[*idx] > self.cartas[*idx].1 {
                    continue 'outer;
                }
            }
            let cartas: Vec<Carta> = indices.iter().map(|idx| self.cartas[*idx].0).collect();
            let freq = contadores
                .iter()
                .zip(self.cartas.iter())
                .filter(|(count, (_, _))| **count > 0)
                .map(|(count, (_, freq))| Integer::from(*freq).binomial(*count as u32))
                .reduce(|acc, v| acc * v)
                .unwrap();
            return Some((cartas, freq.to_usize().unwrap()));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mus::{Carta, DistribucionCartaIter};

    use super::CartaIter;

    #[test]
    fn test_iterator() {
        let cartas = [Carta::As, Carta::Cuatro, Carta::Cinco, Carta::Seis];
        let it = CartaIter::new(&cartas, 2);
        assert_eq!(it.count(), 10);

        let cartas = [
            Carta::As,
            Carta::Cuatro,
            Carta::Cinco,
            Carta::Seis,
            Carta::Siete,
            Carta::Sota,
            Carta::Caballo,
            Carta::Rey,
        ];
        let it = CartaIter::new(&cartas, 4);
        assert_eq!(it.count(), 330);
    }

    #[test]
    fn test_dist_iterator() {
        let cartas = [
            (Carta::As, 1),
            (Carta::Dos, 1),
            (Carta::Tres, 1),
            (Carta::Cuatro, 1),
        ];
        let it = DistribucionCartaIter::new(&cartas, 4);
        assert_eq!(it.count(), 1);
        let cartas = [(Carta::As, 2), (Carta::Cuatro, 1)];
        let it = DistribucionCartaIter::new(&cartas, 2);
        assert_eq!(it.count(), 2);
        let mut it = DistribucionCartaIter::new(&cartas, 2);
        assert_eq!(it.next().unwrap(), (vec![Carta::As, Carta::As], 1));
        assert_eq!(it.next().unwrap(), (vec![Carta::As, Carta::Cuatro], 2));
    }
}
