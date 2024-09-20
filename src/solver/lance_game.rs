use crate::{
    mus::{Accion, Baraja, Carta, Lance, Mano, PartidaMus},
    ActionNode, Game,
};

use super::{BancoEstrategias, TipoEstrategia};

#[derive(Debug)]
pub struct LanceGame {
    manos_normalizadas: [String; 2],
    tipo_estrategia: TipoEstrategia,
    partida: PartidaMus,
    lance: Lance,
    banco_estrategias: Option<BancoEstrategias>,
}

impl LanceGame {
    pub fn new_random(baraja: &Baraja, lance: Lance, tantos: [u8; 2]) -> Self {
        let partida;
        loop {
            let mut b = baraja.clone();
            b.barajar();
            let manos = Self::repartir_manos(b);
            let intento_partida = PartidaMus::new_partida_lance(lance, manos, tantos);
            if let Some(p) = intento_partida {
                partida = p;
                break;
            }
        }
        let (tipo_estrategia, manos_normalizadas) =
            TipoEstrategia::normalizar_mano(partida.manos(), &lance);
        Self {
            partida,
            lance,
            manos_normalizadas,
            tipo_estrategia,
            banco_estrategias: None,
        }
    }

    fn repartir_manos(mut b: Baraja) -> Vec<Mano> {
        let mut manos = Vec::with_capacity(4);
        for _ in 0..4 {
            let mut m = Vec::<Carta>::with_capacity(4);
            for _ in 0..4 {
                m.push(b.repartir().unwrap());
            }
            manos.push(Mano::new(m));
        }
        manos
    }

    pub fn tipo_estrategia(&self) -> TipoEstrategia {
        self.tipo_estrategia
    }

    pub fn train(
        &mut self,
        b: BancoEstrategias,
        action_tree: &ActionNode<usize, Accion>,
    ) -> (BancoEstrategias, [f32; 2]) {
        let banco = b;
        self.banco_estrategias = Some(banco);
        let cfr = self
            .banco_estrategias
            .as_ref()
            .unwrap()
            .estrategia_lance_mut(self.lance, self.tipo_estrategia);
        let mut c = cfr.take();

        let u = [
            c.chance_cfr(self, action_tree, 0, 1., 1.),
            c.chance_cfr(self, action_tree, 1, 1., 1.),
        ];

        cfr.replace(c);
        let banco = self.banco_estrategias.take().unwrap();
        (banco, u)
    }
}

impl Game<usize, Accion> for LanceGame {
    fn utility(&self, player: usize, history: &[Accion]) -> f32 {
        let mut partida = self.partida.clone();
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
        payoff[player] as f32
    }

    fn info_set_str(&self, player: usize, history: &[Accion]) -> String {
        let mut output = String::with_capacity(9 + history.len() + 1);
        output.push_str(&self.manos_normalizadas[player]);
        output.push(',');
        for i in history.iter() {
            output.push_str(&i.to_string());
        }
        output
    }
}
