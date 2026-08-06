use std::ops::Range;

use itertools::{CombinationsWithReplacement, Itertools};

use crate::mus::Baraja;

use super::Carta;

#[cfg(windows)]
fn binomial(n: usize, k: usize) -> usize {
    num_integer::binomial(n, k)
}

#[cfg(not(windows))]
fn binomial(n: usize, k: usize) -> usize {
    rug::Integer::from(n).binomial(k as u32).to_usize().unwrap()
}

/// Iterador de manos de cartas de mus.
///
/// Este iterador asume que las cartas se pueden repetir. Por ejemplo, si
/// tenemos:
///
///     use musolver::mus::{Carta, CartaIter};
///
///     let cartas = [Carta::As, Carta::Cuatro, Carta::Rey];
///     let mut iter = CartaIter::new(&cartas, 2);
///     assert_eq!(iter.count(), 6);
///
/// Las seis parejas que genera son: AA, A4, AR, 44, 4R, RR.
pub struct CartaIter<'a> {
    cartas: &'a [Carta],
    iter: CombinationsWithReplacement<Range<usize>>,
}

impl<'a> CartaIter<'a> {
    /// Crea un nuevo iterador a partir de un slice de Cartas y el número de cartas que se desean
    /// tener en la mano.
    pub fn new(cartas: &'a [Carta], num_cartas: usize) -> Self {
        let iter: CombinationsWithReplacement<Range<usize>> =
            (0..cartas.len()).combinations_with_replacement(num_cartas);
        Self { cartas, iter }
    }
}

impl<'a> Iterator for CartaIter<'a> {
    type Item = Vec<Carta>;

    /// Devuelve la siguiente mano en el iterador.
    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next();
        next.map(|indices| indices.iter().map(|idx| self.cartas[*idx]).collect())
    }
}

#[derive(Clone)]
pub struct CombinationsWithReplacementProb {
    max_frequencies: Vec<usize>,
    current_frequencies: Vec<usize>,
    total_frequency: usize,
    iter: CombinationsWithReplacement<Range<usize>>,
}

/// Combinations with replacement of n elements and a maximum frequency for each element. The
/// iterator returns each combination with its preobability.
impl CombinationsWithReplacementProb {
    /// Creates a new iterator of indices of n elements taken with replacement in groups of k. The
    /// vector max_frequencies stores the maximum frequency for each of the n elements.
    pub fn new(k: usize, max_frequencies: Vec<usize>) -> Self {
        let iter: CombinationsWithReplacement<Range<usize>> =
            (0..max_frequencies.len()).combinations_with_replacement(k);
        let num_elements: usize = max_frequencies.iter().sum();
        let total_frequency = binomial(num_elements, k);
        CombinationsWithReplacementProb {
            iter,
            total_frequency,
            current_frequencies: max_frequencies.clone(),
            max_frequencies,
        }
    }
}

impl Iterator for CombinationsWithReplacementProb {
    type Item = (Vec<usize>, f64);

    fn next(&mut self) -> Option<Self::Item> {
        'outer: loop {
            let next = self.iter.next();
            let indices = next?;

            self.current_frequencies
                .copy_from_slice(&self.max_frequencies);
            for idx in &indices {
                match self.current_frequencies[*idx].checked_sub(1) {
                    None => continue 'outer,
                    Some(r) => self.current_frequencies[*idx] = r,
                }
            }
            let freq = self
                .current_frequencies
                .iter()
                .zip(self.max_frequencies.iter())
                .filter(|(count, max_freq)| **count < **max_freq)
                .map(|(count, max_freq)| binomial(*max_freq, *max_freq - *count))
                .reduce(|acc, v| acc * v)
                .unwrap();
            return Some((indices, freq as f64 / self.total_frequency as f64));
        }
    }
}

pub struct DistribucionCartaIter<const N: usize, const M: usize> {
    cartas: [(Carta, u8); M],
    iter: CombinationsWithReplacementProb,
}

/// Iterador de manos de cartas de mus.
impl<const N: usize, const M: usize> DistribucionCartaIter<N, M> {
    /// Crea un nuevo iterador a partir de una distribución de cartas y el número de cartas que se
    /// desean tener ne la mano. La distribución se indica con un vector de pares (Carta, u8),
    /// donde el entero indica el número de cartas disponibles de ese valor.
    pub fn new(cartas: [(Carta, u8); M]) -> Self {
        let frequencies: Vec<usize> = cartas.iter().map(|(_, f)| *f as usize).collect();
        let iter = CombinationsWithReplacementProb::new(N, frequencies);
        Self { cartas, iter }
    }

    pub fn current_frequencies(&self) -> &[usize] {
        &self.iter.current_frequencies
    }

    pub fn cartas(&self) -> [(Carta, u8); M] {
        self.cartas
    }
}

impl<const N: usize, const M: usize> Iterator for DistribucionCartaIter<N, M> {
    type Item = ([Carta; N], f64);

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.iter.next()?;
        let cartas = std::array::from_fn(|idx| self.cartas[next.0[idx]].0);
        Some((cartas, next.1))
    }
}

pub struct DistribucionDobleCartaIter<const N: usize, const M: usize> {
    cartas: [(Carta, u8); M],
    mano_actual1: Option<([Carta; N], f64)>,
    iter1: CombinationsWithReplacementProb,
    iter2: CombinationsWithReplacementProb,
}
///
/// Iterador de pares de manos de mus.
impl<const N: usize, const M: usize> DistribucionDobleCartaIter<N, M> {
    /// Crea un nuevo iterador a partir de una distribución de cartas y el número de cartas que se
    /// desean tener en cada una de las manos. La distribución se indica con un vector de pares (Carta, u8),
    /// donde el entero indica el número de cartas disponibles de ese valor.
    pub fn new(cartas: [(Carta, u8); M]) -> Self {
        let frecuencias: Vec<usize> = cartas.iter().map(|(_, f)| *f as usize).collect();
        let mut iter1 = CombinationsWithReplacementProb::new(N, frecuencias);
        let idx1 = iter1.next();
        match &idx1 {
            None => Self {
                cartas,
                mano_actual1: None,
                iter2: iter1.clone(),
                iter1,
            },
            Some(ind) => {
                let arr_cartas = std::array::from_fn(|idx| cartas[ind.0[idx]].0);
                let mano_actual1: Option<([Carta; N], f64)> = Some((arr_cartas, ind.1));
                let frecuencias2 = iter1.current_frequencies.clone();
                let iter2 = CombinationsWithReplacementProb::new(N, frecuencias2);
                Self {
                    cartas,
                    mano_actual1,
                    iter1,
                    iter2,
                }
            }
        }
    }

    fn new_iter2(&mut self) {
        let next = self.iter1.next();
        if let Some((idx, frec)) = &next {
            let arr_cartas = std::array::from_fn(|i| self.cartas[idx[i]].0);
            self.mano_actual1 = Some((arr_cartas, *frec));
            let frecuencias2 = self.iter1.current_frequencies.clone();
            self.iter2 = CombinationsWithReplacementProb::new(idx.len(), frecuencias2);
        } else {
            self.mano_actual1 = None;
        }
    }

    pub fn current_frequencies(&self) -> &[usize] {
        &self.iter2.current_frequencies
    }

    pub fn cartas(&self) -> [(Carta, u8); M] {
        self.cartas
    }
}

impl<const N: usize, const M: usize> Iterator for DistribucionDobleCartaIter<N, M> {
    type Item = ([Carta; N], [Carta; N], f64);

    fn next(&mut self) -> Option<Self::Item> {
        let mano1 = self.mano_actual1.as_ref()?;
        let next = self.iter2.next();
        if let Some((idx, frec)) = next {
            let cartas = std::array::from_fn(|i| self.cartas[idx[i]].0);
            Some((mano1.0, cartas, mano1.1 * frec))
        } else {
            self.new_iter2();
            let mano1 = self.mano_actual1.as_ref()?;
            let idx = self.iter2.next().unwrap();
            let cartas = std::array::from_fn(|i| self.cartas[idx.0[i]].0);
            Some((mano1.0, cartas, mano1.1 * idx.1))
        }
    }
}

type DoubleDeal = ([Carta; 4], [Carta; 4], f64, [(Carta, u8); 8]);
pub struct RepartoMusDosJugadoresIter {
    inner: RepartoDosManosMusIter,
    remaining: usize,
}

impl RepartoMusDosJugadoresIter {
    pub fn new() -> Self {
        Self {
            inner: RepartoDosManosMusIter::new(Baraja::FREC_BARAJA_MUS),
            remaining: 104_820,
        }
    }
}

impl Default for RepartoMusDosJugadoresIter {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for RepartoMusDosJugadoresIter {
    type Item = DoubleDeal;

    fn next(&mut self) -> Option<Self::Item> {
        let (mano1, mano2, prob, freq) = self.inner.next()?;

        let mut dist = self.inner.cartas();
        for (d, f) in std::iter::zip(&mut dist, &freq) {
            d.1 = f.1;
        }
        self.remaining -= 1;

        Some((mano1, mano2, prob, dist))
    }

    fn size_hint(&self) -> (usize, std::option::Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for RepartoMusDosJugadoresIter {}

type QuadrupleDeal = (
    [Carta; 4],
    [Carta; 4],
    [Carta; 4],
    [Carta; 4],
    f64,
    [(Carta, u8); 8],
);

pub struct RepartoMusIter {
    outer: RepartoDosManosMusIter,
    /// Reparto actual de las dos primeras manos junto con el iterador sobre las
    /// dos manos restantes que se generan a partir de la distribución sobrante.
    actual: Option<([Carta; 4], [Carta; 4], f64, RepartoDosManosMusIter)>,
    remaining: usize,
}

impl RepartoMusIter {
    pub fn new() -> Self {
        let mut outer = RepartoDosManosMusIter::new(Baraja::FREC_BARAJA_MUS);
        let actual = outer.next().map(|(mano1, mano2, prob, dist)| {
            (mano1, mano2, prob, RepartoDosManosMusIter::new(dist))
        });
        Self {
            outer,
            actual,
            remaining: 7_355_552_285,
        }
    }
}

impl Default for RepartoMusIter {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for RepartoMusIter {
    type Item = QuadrupleDeal;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (mano1, mano2, prob, inner) = self.actual.as_mut()?;
            if let Some((mano3, mano4, prob2, dist2)) = inner.next() {
                self.remaining -= 1;
                return Some((*mano1, *mano2, mano3, mano4, *prob * prob2, dist2));
            }
            // Se han agotado las dos últimas manos para este reparto inicial:
            // avanzamos al siguiente reparto de las dos primeras manos.
            self.actual = self.outer.next().map(|(mano1, mano2, prob, dist)| {
                (mano1, mano2, prob, RepartoDosManosMusIter::new(dist))
            });
        }
    }

    fn size_hint(&self) -> (usize, std::option::Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for RepartoMusIter {}

struct RepartoDosManosMusIter(DistribucionDobleCartaIter<4, 8>);

impl RepartoDosManosMusIter {
    fn new(frequencies: [(Carta, u8); 8]) -> Self {
        Self(DistribucionDobleCartaIter::new(frequencies))
    }
    fn cartas(&self) -> [(Carta, u8); 8] {
        self.0.cartas()
    }
}

impl Iterator for RepartoDosManosMusIter {
    type Item = DoubleDeal;

    fn next(&mut self) -> Option<Self::Item> {
        let (mano1, mano2, prob) = self.0.next()?;

        let mut dist = self.0.cartas();
        for (d, f) in std::iter::zip(&mut dist, self.0.current_frequencies()) {
            d.1 = *f as u8;
        }

        Some((mano1, mano2, prob, dist))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let it = DistribucionCartaIter::<4, 4>::new(cartas);
        assert_eq!(it.count(), 1);

        let cartas = [(Carta::As, 2), (Carta::Cuatro, 1)];
        let it = DistribucionCartaIter::<2, 2>::new(cartas);
        assert_eq!(it.count(), 2);
        let mut it = DistribucionCartaIter::new(cartas);
        assert_eq!(it.next().unwrap(), ([Carta::As, Carta::As], 1. / 3.));
        assert_eq!(it.next().unwrap(), ([Carta::As, Carta::Cuatro], 2. / 3.));
    }

    #[test]
    fn test_double_dist_iterator() {
        let cartas = [
            (Carta::As, 1),
            (Carta::Dos, 1),
            (Carta::Tres, 1),
            (Carta::Cuatro, 1),
        ];
        let it = DistribucionDobleCartaIter::<2, 4>::new(cartas);
        assert_eq!(it.count(), 6);

        let cartas = [(Carta::As, 2), (Carta::Cuatro, 2)];
        let it = DistribucionDobleCartaIter::<2, 2>::new(cartas);
        assert_eq!(it.count(), 3);
        let mut it = DistribucionDobleCartaIter::new(cartas);
        assert_eq!(
            it.next().unwrap(),
            (
                [Carta::As, Carta::As],
                [Carta::Cuatro, Carta::Cuatro],
                1. / 6.
            )
        );
    }

    #[test]
    fn test_current_frequencies() {
        let cartas = [(Carta::As, 2), (Carta::Cuatro, 2)];
        let mut it = DistribucionDobleCartaIter::<2, 2>::new(cartas);
        it.next();
        assert_eq!(it.current_frequencies(), &[0, 0]);
    }

    #[test]
    fn test_repartos_mus() {
        let reparto = RepartoMusDosJugadoresIter::new();
        assert_eq!(reparto.len(), reparto.count());
        let reparto = RepartoMusDosJugadoresIter::new();
        let total_probability = reparto.fold(0., |accum, (_, _, prob, _)| accum + prob);
        assert!((total_probability - 1.).abs() < 1e-9);
    }
}
