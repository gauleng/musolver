use crate::{
    mus::{Accion, Baraja, Lance, PartidaMus},
    Game,
};

use super::TipoEstrategia;

#[derive(Debug)]
pub struct LanceGame {
    lance: Lance,
    tantos: [u8; 2],
    partida: Option<PartidaMus>,
    baraja: Baraja,
    info_set_prefix: Option<[String; 2]>,
    abstract_game: bool,
}

impl LanceGame {
    pub fn new(lance: Lance, tantos: [u8; 2], abstract_game: bool) -> Self {
        let baraja = Baraja::baraja_mus();
        Self {
            lance,
            tantos,
            baraja,
            abstract_game,
            partida: None,
            info_set_prefix: None,
        }
    }

    pub fn from_partida_mus(partida_mus: &PartidaMus, abstract_game: bool) -> Option<Self> {
        Some(Self {
            lance: partida_mus.lance_actual()?,
            tantos: *partida_mus.tantos(),
            baraja: Baraja::baraja_mus(),
            abstract_game,
            partida: Some(partida_mus.clone()),
            info_set_prefix: LanceGame::info_set_prefix(partida_mus, abstract_game),
        })
    }

    fn info_set_prefix(partida_mus: &PartidaMus, abstracto: bool) -> Option<[String; 2]> {
        let lance = partida_mus.lance_actual()?;
        let (tipo_estrategia, manos_normalizadas) =
            TipoEstrategia::normalizar_mano(partida_mus.manos(), &lance);
        let m = if abstracto {
            manos_normalizadas.to_abstract_string_array(&lance)
        } else {
            manos_normalizadas.to_string_array()
        };
        Some([
            tipo_estrategia.to_string() + "," + &m[0],
            tipo_estrategia.to_string() + "," + &m[1],
        ])
    }
}

impl Game<usize, Accion> for LanceGame {
    fn new_random(&mut self) {
        loop {
            self.baraja.barajar();
            let manos = self.baraja.repartir_manos();
            let intento_partida = PartidaMus::new_partida_lance(self.lance, manos, self.tantos);
            if let Some(p) = intento_partida {
                self.info_set_prefix = LanceGame::info_set_prefix(&p, self.abstract_game);
                self.partida = Some(p);
                break;
            }
        }
    }

    fn utility(&self, player: usize, history: &[Accion]) -> f64 {
        let mut partida = self.partida.as_ref().unwrap().clone();
        let turno_inicial = partida.turno().unwrap();
        history.iter().for_each(|&a| {
            let _ = partida.actuar(a);
        });
        let mut tantos = *partida.tantos();
        if turno_inicial == 1 {
            tantos.swap(0, 1);
        }
        let payoff = [
            tantos[0] as i8 - tantos[1] as i8,
            tantos[1] as i8 - tantos[0] as i8,
        ];
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
