use std::cell::RefCell;

use crate::{
    mus::{Accion, Baraja, Carta, Lance, Mano, PartidaMus},
    ActionNode, Game,
};

use super::{BancoEstrategias, TipoEstrategia};

#[derive(Debug)]
pub struct MusGame<'a> {
    manos_normalizadas: [String; 2],
    tipo_estrategia: TipoEstrategia,
    partida: PartidaMus,
    banco_estrategias: Option<RefCell<BancoEstrategias>>,
    action_tree: Option<&'a ActionNode<usize, Accion>>,
    switched: bool,
}

impl<'a> MusGame<'a> {
    pub fn new_random(baraja: &mut Baraja, tantos: [u8; 2]) -> Self {
        baraja.barajar();
        let manos = Self::repartir_manos(baraja);
        let partida = PartidaMus::new(manos, tantos);
        let (tipo_estrategia, manos_normalizadas) =
            TipoEstrategia::normalizar_mano(partida.manos(), &Lance::Grande);
        let manos_normalizadas_str = manos_normalizadas.to_abstract_string_array(&Lance::Grande);
        Self {
            partida,
            manos_normalizadas: manos_normalizadas_str,
            tipo_estrategia,
            banco_estrategias: None,
            action_tree: None,
            switched: false,
        }
    }

    fn from_partida_mus(partida: PartidaMus) -> Self {
        let (tipo_estrategia, manos_normalizadas) =
            TipoEstrategia::normalizar_mano(partida.manos(), &partida.lance_actual().unwrap());
        let tantos = partida.tantos();
        let pareja_mano = partida.turno().unwrap();
        let prefijo = format!("{}:{},", tantos[pareja_mano], tantos[1 - pareja_mano]);
        let mut info_set = [prefijo.clone(), prefijo.clone()];
        let manos_normalizadas_str =
            manos_normalizadas.to_abstract_string_array(&partida.lance_actual().unwrap());
        info_set[0].push_str(&manos_normalizadas_str[0]);
        info_set[1].push_str(&manos_normalizadas_str[1]);
        Self {
            partida,
            manos_normalizadas: info_set,
            tipo_estrategia,
            banco_estrategias: None,
            action_tree: None,
            switched: false,
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

    pub fn tipo_estrategia(&self) -> TipoEstrategia {
        self.tipo_estrategia
    }

    pub fn train(
        &mut self,
        b: BancoEstrategias,
        action_tree: &'a ActionNode<usize, Accion>,
    ) -> (BancoEstrategias, [f64; 2]) {
        self.banco_estrategias = Some(RefCell::new(b));
        self.action_tree = Some(action_tree);
        let banco = self.banco_estrategias.as_ref().unwrap().borrow();
        let cfr = banco.estrategia_lance_mut(Lance::Grande, self.tipo_estrategia);
        let mut c = cfr.take();
        drop(banco);

        let u = [
            c.chance_cfr(self, self.action_tree.as_ref().unwrap(), 0, 1., 1.),
            c.chance_cfr(self, self.action_tree.as_ref().unwrap(), 1, 1., 1.),
        ];

        let banco = self.banco_estrategias.as_ref().unwrap().take();
        let cfr = banco.estrategia_lance_mut(Lance::Grande, self.tipo_estrategia);
        cfr.replace(c);
        (banco, u)
    }
}

impl<'a> Game<usize, Accion> for MusGame<'a> {
    fn utility(&self, player: usize, history: &[Accion]) -> f64 {
        let mut partida = self.partida.clone();
        history.iter().for_each(|&a| {
            let _ = partida.actuar(a);
        });

        if let Some(lance) = partida.lance_actual() {
            let action_tree = self.action_tree.unwrap();

            let mut trainer = MusGame::from_partida_mus(partida);
            let banco = self.banco_estrategias.as_ref().unwrap().take();
            let cfr = banco.estrategia_lance_mut(lance, trainer.tipo_estrategia);
            let mut c = cfr.take();
            trainer.banco_estrategias = Some(RefCell::new(banco));
            trainer.action_tree = Some(action_tree);
            let mut acting_player = player;
            trainer
                .partida
                .turno()
                .iter()
                .zip(self.partida.turno().iter())
                .for_each(|(t, s)| {
                    if *t != *s {
                        trainer.switched = !self.switched;
                        acting_player = 1 - acting_player;
                    }
                });
            let u = c.chance_cfr(&trainer, action_tree, acting_player, 1., 1.);

            let banco = trainer.banco_estrategias.as_ref().unwrap().take();
            let cfr = banco.estrategia_lance_mut(lance, trainer.tipo_estrategia);
            cfr.replace(c);
            self.banco_estrategias.as_ref().unwrap().replace(banco);

            u
        } else {
            let mut tantos = *partida.tantos();

            let payoff = [
                tantos[0] as i8 - tantos[1] as i8,
                tantos[1] as i8 - tantos[0] as i8,
            ];

            if self.switched {
                tantos.swap(0, 1);
            }
            // println!(
            //     "Tantos para el jugador {}  con acciones {:?}: {}",
            //     player, self.history, tantos[player]
            // );
            payoff[player] as f64
        }
    }

    fn info_set_str(&self, player: usize, history: &[Accion]) -> String {
        let mut output = String::with_capacity(15 + history.len() + 1);
        output.push_str(&self.manos_normalizadas[player]);
        output.push(',');
        for i in history.iter() {
            output.push_str(&i.to_string());
        }
        output
    }
}
