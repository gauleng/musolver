use super::Mano;

use std::cmp;

trait Lance {
    fn compara_manos(a: &Mano, b: &Mano) -> cmp::Ordering;
}

struct Grande {}

impl Lance for Grande {
    fn compara_manos(a: &Mano, b: &Mano) -> cmp::Ordering {
        cmp::Ordering::Equal
    }
}
