use crate::mus::Mano;

use std::cmp;

pub enum Lance {
    Grande,
    Chica,
    Pares,
    Punto,
    Juego,
}

impl Lance {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        match self {
            Lance::Grande => a.codigo().cmp(&b.codigo()),
            Lance::Chica => b.codigo().cmp(&a.codigo()),
            Lance::Pares => a.num_parejas().cmp(&b.num_parejas()),
            Lance::Juego => a.codigo().cmp(&b.codigo()),
            Lance::Punto => a.puntos().cmp(&b.puntos()),
        }
    }

    /// Dado un vector de manos de mus, devuelve el índice de la mejor de ellas dado el lance en
    /// juego.
    pub fn mejor_mano(&self, manos: &Vec<Mano>) -> usize {
        let mut indices: Vec<usize> = (0..manos.len()).collect();
        indices.sort_by(|i, j| self.compara_manos(&manos[*i], &manos[*j]));
        *indices.last().unwrap()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn name() {
        let a = Mano::try_from("1147").unwrap();
        let b = Mano::try_from("1247").unwrap();
        let grande = Lance::Grande;
        assert_eq!(grande.compara_manos(&a, &b), std::cmp::Ordering::Equal);
        let manos = vec![a, b];
        grande.mejor_mano(&manos);
    }
}
