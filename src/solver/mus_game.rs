use std::cell::RefCell;

use crate::{
    mus::{Accion, Baraja, Carta, Lance, Mano, PartidaMus, Turno},
    ActionNode, Game,
};

use super::{BancoEstrategias, HandConfiguration, ManosNormalizadas};

#[derive(Debug)]
pub struct MusGame<'a> {
    manos_normalizadas: [String; 2],
    tipo_estrategia: HandConfiguration,
    partida: PartidaMus,
    banco_estrategias: Option<RefCell<BancoEstrategias>>,
    action_tree: Option<&'a ActionNode<usize, Accion>>,
    switched: bool,
    history: Vec<Accion>,
}

impl<'a> MusGame<'a> {
    pub fn new(baraja: &mut Baraja, tantos: [u8; 2]) -> Self {
        baraja.barajar();
        let manos = Self::repartir_manos(baraja);
        let partida = PartidaMus::new(manos, tantos);
        let manos_normalizadas =
            ManosNormalizadas::normalizar_mano(partida.manos(), &Lance::Grande);
        let manos_normalizadas_str = manos_normalizadas.to_string_array();
        let tipo_estrategia = manos_normalizadas.hand_configuration();
        Self {
            partida,
            manos_normalizadas: manos_normalizadas_str,
            tipo_estrategia,
            banco_estrategias: None,
            action_tree: None,
            switched: false,
            history: vec![],
        }
    }

    fn from_partida_mus(partida: PartidaMus) -> Self {
        let manos_normalizadas =
            ManosNormalizadas::normalizar_mano(partida.manos(), &partida.lance_actual().unwrap());
        let tantos = partida.tantos();
        let pareja_mano = match partida.turno().unwrap() {
            Turno::Jugador(id) => id,
            Turno::Pareja(id) => id,
        } as usize;
        let prefijo = format!("{}:{},", tantos[pareja_mano], tantos[1 - pareja_mano]);
        let manos_normalizadas_str = manos_normalizadas.to_string_array();
        let mut info_set = [prefijo.clone(), prefijo.clone()];
        info_set[0].push_str(&manos_normalizadas_str[0]);
        info_set[1].push_str(&manos_normalizadas_str[1]);
        let tipo_estrategia = manos_normalizadas.hand_configuration();
        Self {
            partida,
            manos_normalizadas: info_set,
            tipo_estrategia,
            banco_estrategias: None,
            action_tree: None,
            switched: false,
            history: vec![],
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

    pub fn tipo_estrategia(&self) -> HandConfiguration {
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
        let cfr = banco.estrategia_lance_mut(Lance::Grande);
        let c = cfr.take();
        drop(banco);

        let u = [
            0.,
            0., // c.chance_sampling(self, self.action_tree.as_ref().unwrap(), 0, 1., 1.),
               // c.chance_sampling(self, self.action_tree.as_ref().unwrap(), 1, 1., 1.),
        ];

        let banco = self.banco_estrategias.as_ref().unwrap().take();
        let cfr = banco.estrategia_lance_mut(Lance::Grande);
        cfr.replace(c);
        (banco, u)
    }
}

impl<'a> Game<usize, Accion> for MusGame<'a> {
    fn utility(&self, player: usize) -> f64 {
        let mut partida = self.partida.clone();
        self.history.iter().for_each(|&a| {
            let _ = partida.actuar(a);
        });

        if let Some(lance) = partida.lance_actual() {
            let action_tree = self.action_tree.unwrap();

            let mut trainer = MusGame::from_partida_mus(partida);
            let banco = self.banco_estrategias.as_ref().unwrap().take();
            let cfr = banco.estrategia_lance_mut(lance);
            let c = cfr.take();
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
            let u = 0.; //c.chance_sampling(&trainer, action_tree, acting_player, 1., 1.);

            let banco = trainer.banco_estrategias.as_ref().unwrap().take();
            let cfr = banco.estrategia_lance_mut(lance);
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

    fn info_set_str(&self, player: usize) -> String {
        let mut output = String::with_capacity(15 + self.history.len() + 1);
        output.push_str(&self.manos_normalizadas[player]);
        output.push(',');
        for i in self.history.iter() {
            output.push_str(&i.to_string());
        }
        output
    }

    fn new_random(&mut self) {
        todo!()
    }

    fn new_iter<F>(&mut self, _f: F)
    where
        F: FnMut(&mut Self, f64),
    {
        todo!()
    }

    fn num_players(&self) -> usize {
        2
    }

    fn player_id(&self, idx: usize) -> usize {
        idx
    }

    fn actions(&self) -> Option<Vec<Accion>> {
        todo!()
    }

    fn is_terminal(&self) -> bool {
        todo!()
    }

    fn current_player(&self) -> Option<usize> {
        todo!()
    }

    fn act(&mut self, _a: Accion) {
        todo!()
    }

    fn takeback(&mut self) {
        todo!()
    }
}
