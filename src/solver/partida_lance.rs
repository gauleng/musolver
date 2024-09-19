use crate::{
    mus::{Accion, Baraja, Carta, Juego, Lance, Mano, Pares, PartidaMus},
    Game,
};

#[derive(Debug, Clone, Copy)]
pub enum TipoEstrategia {
    CuatroManos = 0,
    TresManos1vs2 = 1,
    TresManos1vs2Intermedio = 2,
    TresManos2vs1 = 3,
    DosManos = 4,
}

impl TipoEstrategia {
    fn normalizar_mano(m: &[Mano], l: &Lance) -> (Self, [String; 2]) {
        match l {
            Lance::Grande | Lance::Chica | Lance::Punto => {
                let m1 = m[0].to_string() + "," + &m[2].to_string();
                let m2 = m[1].to_string() + "," + &m[3].to_string();
                (TipoEstrategia::CuatroManos, [m1, m2])
            }
            Lance::Pares => {
                let jugadas: Vec<Option<Pares>> = m.iter().map(|m| m.pares()).collect();
                Self::normalizar_mano_jugadas(m, &jugadas)
            }
            Lance::Juego => {
                let jugadas: Vec<Option<Juego>> = m.iter().map(|m| m.juego()).collect();
                Self::normalizar_mano_jugadas(m, &jugadas)
            }
        }
    }

    fn normalizar_mano_jugadas<T>(m: &[Mano], jugadas: &[Option<T>]) -> (Self, [String; 2]) {
        let mut parejas = [Vec::new(), Vec::new()];
        jugadas.iter().enumerate().for_each(|(i, p)| {
            if p.is_some() {
                parejas[i % 2].push(&m[i]);
            }
        });
        if jugadas[1].is_some() && jugadas[2].is_some() && jugadas[3].is_none() {
            parejas.swap(0, 1);
        }
        if parejas[0].len() == 2 && parejas[1].len() == 2 {
            let m1 = m[0].to_string() + "," + &m[2].to_string();
            let m2 = m[1].to_string() + "," + &m[3].to_string();
            (TipoEstrategia::CuatroManos, [m1, m2])
        } else if parejas[0].len() == 1 && parejas[1].len() == 1 {
            (
                TipoEstrategia::DosManos,
                [
                    parejas[0][0].to_string() + ",",
                    parejas[1][0].to_string() + ",",
                ],
            )
        } else if parejas[0].len() == 1 && parejas[1].len() == 2 {
            let tipo_estrategia = if jugadas[2].is_none() {
                TipoEstrategia::TresManos1vs2
            } else {
                TipoEstrategia::TresManos1vs2Intermedio
            };
            (
                tipo_estrategia,
                [
                    parejas[0][0].to_string() + ",",
                    parejas[1][0].to_string() + "," + &parejas[1][1].to_string(),
                ],
            )
        } else {
            (
                TipoEstrategia::TresManos2vs1,
                [
                    parejas[0][0].to_string() + "," + &parejas[0][1].to_string(),
                    parejas[1][0].to_string() + ",",
                ],
            )
        }
    }
}

#[derive(Debug)]
pub struct PartidaLance {
    manos_normalizadas: [String; 2],
    tipo_estrategia: TipoEstrategia,
    partida: PartidaMus,
    lance: Lance,
}

impl PartidaLance {
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
}

impl Game<usize, Accion> for PartidaLance {
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
