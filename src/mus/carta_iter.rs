use super::Carta;

pub struct CartaIter<'a> {
    cartas: &'a [Carta],
    indices: Vec<usize>,
}

impl<'a> CartaIter<'a> {
    pub fn new(cartas: &'a [Carta], num_cartas: usize) -> Self {
        Self {
            cartas,
            indices: vec![0; num_cartas],
        }
    }
}

impl<'a> Iterator for CartaIter<'a> {
    type Item = Vec<Carta>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.indices[0] == self.cartas.len() {
            return None;
        }

        let siguientes_cartas: Vec<Carta> = self.indices.iter().map(|i| self.cartas[*i]).collect();

        let mut nivel: usize = 0;
        while nivel < self.indices.len() {
            self.indices[nivel] += 1;
            if self.indices[nivel] == self.cartas.len() {
                nivel += 1;
            } else {
                break;
            }
        }
        if nivel < self.indices.len() {
            while nivel > 0 {
                self.indices[nivel - 1] = self.indices[nivel];
                nivel -= 1;
            }
        }
        Some(siguientes_cartas)
    }
}

#[cfg(test)]
mod tests {
    use crate::mus::Carta;

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
}
