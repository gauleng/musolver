use std::ops::Range;
use std::sync::LazyLock;

use itertools::{CombinationsWithReplacement, Itertools};

use crate::mus::Baraja;

use super::Carta;

/// Mayor n con número combinatorio precalculado. Cubre de sobra la baraja de mus (40 cartas), que
/// es el caso que recorren los iteradores de reparto millones de veces. Ampliarlo tiene un coste:
/// la tabla crece de forma cuadrática y deja de caber en la caché de primer nivel.
const MAX_BINOMIAL: usize = 40;

static TABLA_BINOMIAL: LazyLock<[[usize; MAX_BINOMIAL + 1]; MAX_BINOMIAL + 1]> =
    LazyLock::new(|| {
        let mut t = [[0usize; MAX_BINOMIAL + 1]; MAX_BINOMIAL + 1];
        for n in 0..=MAX_BINOMIAL {
            for k in 0..=MAX_BINOMIAL {
                t[n][k] = num_integer::binomial(n, k);
            }
        }
        t
    });

/// Número combinatorio de n sobre k.
///
/// Consulta la tabla precalculada mientras n lo permita y delega en num_integer en caso contrario,
/// de forma que los iteradores siguen sirviendo para barajas de cualquier tamaño.
fn binomial(n: usize, k: usize) -> usize {
    if n <= MAX_BINOMIAL && k <= MAX_BINOMIAL {
        return TABLA_BINOMIAL[n][k];
    }
    num_integer::binomial(n, k)
}

/// Índices en `cartas` de cada carta de la mano. `None` si alguna no está en la distribución.
fn indices_de<const M: usize>(cartas: &[(Carta, u8); M], mano: &[Carta]) -> Option<Vec<usize>> {
    mano.iter()
        .map(|carta| cartas.iter().position(|(c, _)| c == carta))
        .collect()
}

/// Número de repartos que dan exactamente esta mano, junto con las frecuencias que quedan tras
/// retirarla. `None` si la mano no cabe en la distribución.
fn combinaciones_mano(frecuencias: &[usize], indices: &[usize]) -> Option<(usize, Vec<usize>)> {
    let mut restantes = frecuencias.to_vec();
    for idx in indices {
        restantes[*idx] = restantes[*idx].checked_sub(1)?;
    }
    let combinaciones = restantes
        .iter()
        .zip(frecuencias)
        .filter(|(restante, max)| restante < max)
        .map(|(restante, max)| binomial(*max, *max - *restante))
        .reduce(|acum, v| acum * v)
        .unwrap_or(1);
    Some((combinaciones, restantes))
}

/// Probabilidad de repartir exactamente esta mano.
///
/// Es la misma que devuelve [`DistribucionCartaIter`] para esa mano, en forma cerrada para no
/// tener que recorrer el iterador entero.
pub fn probabilidad_mano<const M: usize>(cartas: [(Carta, u8); M], mano: &[Carta]) -> f64 {
    let frecuencias: Vec<usize> = cartas.iter().map(|(_, f)| *f as usize).collect();
    let Some(indices) = indices_de(&cartas, mano) else {
        return 0.;
    };
    let Some((combinaciones, _)) = combinaciones_mano(&frecuencias, &indices) else {
        return 0.;
    };
    combinaciones as f64 / binomial(frecuencias.iter().sum(), mano.len()) as f64
}

/// Probabilidad conjunta de repartir las dos manos, la misma que devuelve
/// [`DistribucionDobleCartaIter`]. La segunda se reparte con las cartas que deja la primera, así
/// que no es el producto de las dos probabilidades por separado.
pub fn probabilidad_dos_manos<const M: usize>(
    cartas: [(Carta, u8); M],
    mano1: &[Carta],
    mano2: &[Carta],
) -> f64 {
    let frecuencias: Vec<usize> = cartas.iter().map(|(_, f)| *f as usize).collect();
    let (Some(indices1), Some(indices2)) = (indices_de(&cartas, mano1), indices_de(&cartas, mano2))
    else {
        return 0.;
    };
    let Some((combinaciones1, restantes)) = combinaciones_mano(&frecuencias, &indices1) else {
        return 0.;
    };
    let Some((combinaciones2, _)) = combinaciones_mano(&restantes, &indices2) else {
        return 0.;
    };
    let total1 = binomial(frecuencias.iter().sum(), mano1.len());
    let total2 = binomial(restantes.iter().sum(), mano2.len());
    (combinaciones1 as f64 / total1 as f64) * (combinaciones2 as f64 / total2 as f64)
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
pub struct CombinationsWithReplacementProb<const K: usize> {
    max_frequencies: Vec<usize>,
    current_frequencies: Vec<usize>,
    total_frequency: usize,
    indices: [usize; K],
    agotado: bool,
    primera: bool,
}

/// Combinations with replacement of n elements and a maximum frequency for each element. The
/// iterator returns each combination with its preobability.
impl<const K: usize> CombinationsWithReplacementProb<K> {
    /// Creates a new iterator of indices of n elements taken with replacement in groups of K. The
    /// vector max_frequencies stores the maximum frequency for each of the n elements.
    pub fn new(max_frequencies: Vec<usize>) -> Self {
        let num_elements: usize = max_frequencies.iter().sum();
        CombinationsWithReplacementProb {
            total_frequency: binomial(num_elements, K),
            current_frequencies: max_frequencies.clone(),
            agotado: max_frequencies.is_empty() || num_elements < K,
            max_frequencies,
            indices: [0; K],
            primera: true,
        }
    }

    /// Avanza al siguiente conjunto de índices no decreciente. Devuelve false si se han agotado.
    fn avanzar(&mut self) -> bool {
        if self.primera {
            self.primera = false;
            return true;
        }
        let n = self.max_frequencies.len();
        for i in (0..K).rev() {
            if self.indices[i] + 1 < n {
                let v = self.indices[i] + 1;
                self.indices[i..].fill(v);
                return true;
            }
        }
        false
    }
}

impl<const K: usize> Iterator for CombinationsWithReplacementProb<K> {
    type Item = ([usize; K], f64);

    fn next(&mut self) -> Option<Self::Item> {
        'outer: loop {
            if self.agotado || !self.avanzar() {
                self.agotado = true;
                return None;
            }

            self.current_frequencies
                .copy_from_slice(&self.max_frequencies);
            for idx in &self.indices {
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
            return Some((self.indices, freq as f64 / self.total_frequency as f64));
        }
    }
}

pub struct DistribucionCartaIter<const N: usize, const M: usize> {
    cartas: [(Carta, u8); M],
    iter: CombinationsWithReplacementProb<N>,
}

/// Iterador de manos de cartas de mus.
impl<const N: usize, const M: usize> DistribucionCartaIter<N, M> {
    /// Crea un nuevo iterador a partir de una distribución de cartas y el número de cartas que se
    /// desean tener ne la mano. La distribución se indica con un vector de pares (Carta, u8),
    /// donde el entero indica el número de cartas disponibles de ese valor.
    pub fn new(cartas: [(Carta, u8); M]) -> Self {
        let frequencies: Vec<usize> = cartas.iter().map(|(_, f)| *f as usize).collect();
        let iter = CombinationsWithReplacementProb::new(frequencies);
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
    iter1: CombinationsWithReplacementProb<N>,
    iter2: CombinationsWithReplacementProb<N>,
}
///
/// Iterador de pares de manos de mus.
impl<const N: usize, const M: usize> DistribucionDobleCartaIter<N, M> {
    /// Crea un nuevo iterador a partir de una distribución de cartas y el número de cartas que se
    /// desean tener en cada una de las manos. La distribución se indica con un vector de pares (Carta, u8),
    /// donde el entero indica el número de cartas disponibles de ese valor.
    pub fn new(cartas: [(Carta, u8); M]) -> Self {
        let frecuencias: Vec<usize> = cartas.iter().map(|(_, f)| *f as usize).collect();
        let mut iter1 = CombinationsWithReplacementProb::new(frecuencias);
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
                let iter2 = CombinationsWithReplacementProb::new(frecuencias2);
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
            self.iter2 = CombinationsWithReplacementProb::new(frecuencias2);
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

pub struct RepartoDescarteMusIter<const N: usize>(DistribucionCartaIter<N, 8>);

impl<const N: usize> RepartoDescarteMusIter<N> {
    pub fn new(frequencies: [(Carta, u8); 8]) -> Self {
        Self(DistribucionCartaIter::new(frequencies))
    }

    pub fn cartas(&self) -> [(Carta, u8); 8] {
        self.0.cartas()
    }
}

impl<const N: usize> Iterator for RepartoDescarteMusIter<N> {
    type Item = ([Carta; N], f64, [(Carta, u8); 8]);

    fn next(&mut self) -> Option<Self::Item> {
        let (mano, prob) = self.0.next()?;

        let mut dist = self.0.cartas();
        for (d, f) in std::iter::zip(&mut dist, self.0.current_frequencies()) {
            d.1 = *f as u8;
        }

        Some((mano, prob, dist))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tabla_binomial() {
        // La tabla precalculada debe coincidir en todo su rango con el cálculo de referencia,
        // incluyendo los k mayores que n, que valen cero.
        for n in 0..=MAX_BINOMIAL {
            for k in 0..=MAX_BINOMIAL {
                assert_eq!(binomial(n, k), num_integer::binomial(n, k), "n={n}, k={k}");
            }
        }
        // Fuera de la tabla se sigue obteniendo el valor correcto.
        assert_eq!(binomial(52, 5), 2_598_960);
    }

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

    /// La forma cerrada tiene que dar exactamente lo mismo que el iterador para todas las manos.
    #[test]
    fn probabilidad_mano_coincide_con_el_iterador() {
        let mut manos = 0;
        for (cartas, prior) in DistribucionCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS) {
            let cerrada = probabilidad_mano(Baraja::FREC_BARAJA_MUS, &cartas);
            assert!(
                (cerrada - prior).abs() < 1e-12,
                "{cartas:?}: {cerrada} != {prior}"
            );
            manos += 1;
        }
        assert!(manos > 300);
    }

    #[test]
    fn probabilidad_dos_manos_coincide_con_el_iterador() {
        let mut pares = 0;
        for (cartas1, cartas2, prior) in
            DistribucionDobleCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS)
        {
            let cerrada = probabilidad_dos_manos(Baraja::FREC_BARAJA_MUS, &cartas1, &cartas2);
            assert!(
                (cerrada - prior).abs() < 1e-12,
                "{cartas1:?} {cartas2:?}: {cerrada} != {prior}"
            );
            pares += 1;
        }
        assert!(pares > 1000);
    }

    /// Una mano que no cabe en la baraja tiene probabilidad cero.
    #[test]
    fn probabilidad_de_una_mano_imposible_es_cero() {
        // Solo hay cuatro Sotas.
        let cinco_sotas = [Carta::Sota; 5];
        assert_eq!(probabilidad_mano(Baraja::FREC_BARAJA_MUS, &cinco_sotas), 0.);
        // Ocho Reyes en total: dos manos de cuatro son posibles, tres no.
        let reyes = [Carta::Rey; 4];
        assert!(probabilidad_dos_manos(Baraja::FREC_BARAJA_MUS, &reyes, &reyes) > 0.);
        let sotas = [Carta::Sota; 4];
        assert_eq!(
            probabilidad_dos_manos(Baraja::FREC_BARAJA_MUS, &sotas, &sotas),
            0.
        );
    }
}
