use crate::mus::Mano;

use std::cmp;

pub trait Lance {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering;

    fn mejor_mano(&self, manos: &Vec<Mano>) -> usize {
        let mut indices: Vec<usize> = (0..manos.len()).collect();
        indices.sort_by(|i, j| self.compara_manos(&manos[*i], &manos[*j]));
        *indices.last().unwrap()
    }
}

pub struct Grande {}

impl Lance for Grande {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        a.codigo().cmp(&b.codigo())
    }
}

pub struct Chica {}

impl Lance for Chica {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        b.codigo().cmp(&a.codigo())
    }
}

pub struct Pares {}

impl Lance for Pares {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        a.num_parejas().cmp(&b.num_parejas())
    }
}
pub struct Punto {}

impl Lance for Punto {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        a.puntos().cmp(&b.puntos())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn name() {
        let a = Mano::try_from("1147").unwrap();
        let b = Mano::try_from("1247").unwrap();
        let grande = Grande {};
        assert_eq!(grande.compara_manos(&a, &b), std::cmp::Ordering::Equal);
        let manos = vec![a, b];
        grande.mejor_mano(&manos);
    }
}
