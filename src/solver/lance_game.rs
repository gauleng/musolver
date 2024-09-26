use crate::{
    mus::{Accion, Baraja, Carta, Lance, Mano, PartidaMus},
    Game,
};

use super::TipoEstrategia;

#[derive(Debug)]
pub struct LanceGame {
    lance: Lance,
    tantos: [u8; 2],
    partida: Option<PartidaMus>,
    info_set_prefix: Option<[String; 2]>,
    baraja: Baraja,
    abstracto: bool,
}

impl LanceGame {
    pub fn new(lance: Lance, tantos: [u8; 2], abstracto: bool) -> Self {
        let baraja = Baraja::baraja_mus();
        Self {
            lance,
            tantos,
            baraja,
            abstracto,
            partida: None,
            info_set_prefix: None,
        }
    }

    fn repartir_manos(b: &Baraja) -> [Mano; 4] {
        let mut c = b.primeras_n_cartas(16).iter();
        core::array::from_fn(|_| {
            let mut m = Vec::<Carta>::with_capacity(4);
            for _ in 0..4 {
                m.push(*c.next().unwrap());
            }
            Mano::new(m)
        })
    }
}

impl Game<usize, Accion> for LanceGame {
    fn new_random(&mut self) {
        loop {
            self.baraja.barajar();
            let manos = Self::repartir_manos(&self.baraja);
            let intento_partida = PartidaMus::new_partida_lance(self.lance, manos, self.tantos);
            if let Some(p) = intento_partida {
                let (tipo_estrategia, manos_normalizadas) =
                    TipoEstrategia::normalizar_mano(p.manos(), &self.lance);
                let m = if self.abstracto {
                    manos_normalizadas.to_abstract_string_array(&self.lance)
                } else {
                    manos_normalizadas.to_string_array()
                };
                self.info_set_prefix = Some([
                    tipo_estrategia.to_string() + "," + &m[0],
                    tipo_estrategia.to_string() + "," + &m[1],
                ]);
                self.partida = Some(p);
                break;
            }
        }
    }

    fn utility(&self, player: usize, history: &[Accion]) -> f64 {
        let mut partida = self.partida.as_ref().unwrap().clone();
        history.iter().for_each(|&a| {
            let _ = partida.actuar(a);
        });
        let turno_inicial = self.lance.turno_inicial(partida.manos());
        let mut tantos = *partida.tantos();

        if turno_inicial == 1 {
            tantos.swap(0, 1);
        }
        let payoff = [
            tantos[0] as i8 - tantos[1] as i8,
            tantos[1] as i8 - tantos[0] as i8,
        ];
        // println!(
        //     "Tantos para el jugador {}  con acciones {:?}: {}",
        //     player, self.history, tantos[player]
        // );
        payoff[player] as f64
    }

    fn info_set_str(&self, player: usize, history: &[Accion]) -> String {
        let mut output = String::with_capacity(9 + history.len() + 1);
        output.push_str(&self.info_set_prefix.as_ref().unwrap()[player]);
        output.push(',');
        for i in history.iter() {
            output.push_str(&i.to_string());
        }
        output
    }
}
