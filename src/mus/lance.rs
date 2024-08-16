use super::Accion;
use super::Mano;

use std::cmp;

trait Lance {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering;

    fn mejor_mano(&self, manos: &Vec<Mano>) -> usize {
        let mut indices: Vec<usize> = (0..manos.len()).collect();
        indices.sort_by(|i, j| self.compara_manos(&manos[*i], &manos[*j]));
        *indices.last().unwrap()
    }
}

struct Grande {}

impl Lance for Grande {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        a.codigo().cmp(&b.codigo())
    }
}

struct Chica {}

impl Lance for Chica {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        b.codigo().cmp(&a.codigo())
    }
}

struct Pares {}

impl Lance for Pares {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        todo!();
    }
}
struct JuegoPunto {}

impl Lance for JuegoPunto {
    fn compara_manos(&self, a: &Mano, b: &Mano) -> cmp::Ordering {
        todo!();
    }
}

fn tantos(lance: &dyn Lance, manos: &Vec<Mano>, acciones: &Vec<Accion>) -> Vec<u8> {
    let ganador = lance.mejor_mano(manos);
    let apostado = acciones.iter().fold(0, |acc, a| match a {
        Accion::Envido(e) => acc + e,
        _ => acc,
    });
    let pareja = ganador % 2;

    let mut tantos = vec![0; manos.len()];
    tantos[pareja] = apostado;
    tantos[2 + pareja] = apostado;
    tantos
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn name() {
        let a = Mano::try_from("1147").unwrap();
        let b = Mano::try_from("1247").unwrap();
        let grande = Grande {};
        assert_eq!(grande.compara_manos(&a, &b), std::cmp::Ordering::Less);
        let chica = Chica {};
        let pares = Pares {};
        let juego = JuegoPunto {};
        let manos = vec![a, b];
        grande.mejor_mano(&manos);
    }
}
