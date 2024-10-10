use std::fmt::Display;

use itertools::Itertools;

use crate::{
    mus::{Accion, Baraja, DistribucionDobleCartaIter, Lance, Mano, PartidaMus},
    solver::ManosNormalizadas,
    Game,
};

use super::HandConfiguration;

pub struct InfoSet {
    pub tipo_estrategia: HandConfiguration,
    pub tantos: [u8; 2],
    pub manos: (Mano, Option<Mano>),
    pub history: Vec<Accion>,
    pub abstract_game: Option<Lance>,
}

impl Display for InfoSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{},{},{},{}",
            self.tantos[0],
            self.tantos[1],
            self.tipo_estrategia,
            if let Some(lance) = self.abstract_game {
                ManosNormalizadas::par_manos_to_abstract_string(&self.manos, &lance)
            } else {
                ManosNormalizadas::par_manos_to_string(&self.manos)
            },
            self.history
                .iter()
                .map(|accion| accion.to_string())
                .join("")
        )
    }
}

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
            HandConfiguration::normalizar_mano(partida_mus.manos(), &lance);
        let info_set_prefix: [String; 2] = core::array::from_fn(|i| {
            InfoSet {
                tantos: *partida_mus.tantos(),
                tipo_estrategia,
                manos: manos_normalizadas.manos(i).to_owned(),
                history: vec![],
                abstract_game: if abstracto { Some(lance) } else { None },
            }
            .to_string()
        });
        Some(info_set_prefix)
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

    fn new_iter<F>(&mut self, mut f: F)
    where
        F: FnMut(&Self, f64),
    {
        let mut iter = DistribucionDobleCartaIter::new(&Baraja::FREC_BARAJA_MUS, 4);
        let mut frecuencia_baraja_2 = Baraja::FREC_BARAJA_MUS;
        while let Some(mano_pareja1) = iter.next() {
            let frequencies2 = iter.current_frequencies();
            frecuencia_baraja_2
                .iter_mut()
                .zip(frequencies2.iter())
                .for_each(|(carta, f2)| {
                    carta.1 = *f2 as u8;
                });
            let iter2 = DistribucionDobleCartaIter::new(&frecuencia_baraja_2, 4);
            for mano_pareja2 in iter2 {
                let manos = [
                    Mano::new(mano_pareja1.0.clone()),
                    Mano::new(mano_pareja2.0.clone()),
                    Mano::new(mano_pareja1.1.clone()),
                    Mano::new(mano_pareja2.1.clone()),
                ];
                let intento_partida = PartidaMus::new_partida_lance(self.lance, manos, self.tantos);
                if let Some(p) = intento_partida {
                    self.info_set_prefix = LanceGame::info_set_prefix(&p, self.abstract_game);
                    self.partida = Some(p);
                    f(self, mano_pareja1.2 * mano_pareja2.2);
                }
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
        for i in history.iter() {
            output.push_str(&i.to_string());
        }
        output
    }

    fn player_id(&self, idx: usize) -> usize {
        idx
    }

    fn num_players(&self) -> usize {
        2
    }
}
