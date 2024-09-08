use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use std::{collections::HashMap, fs::File, io::Write};

use crate::mus::{self, Accion, Apuesta, Baraja, Carta, EstadoLance, Juego, Lance, Mano, Pares};

use super::ActionNode;

#[derive(Debug, Clone)]
pub struct Node {
    regret_sum: Vec<f32>,
    strategy: Vec<f32>,
    strategy_sum: Vec<f32>,
}

impl Node {
    fn new(num_actions: usize) -> Self {
        Self {
            regret_sum: vec![0.; num_actions],
            strategy: vec![0.; num_actions],
            strategy_sum: vec![0.; num_actions],
        }
    }

    pub fn get_strategy(&mut self) -> &Vec<f32> {
        for i in 0..self.strategy.len() {
            self.strategy[i] = self.regret_sum[i].max(0.);
        }
        let normalizing_sum: f32 = self.strategy.iter().sum();
        for i in 0..self.strategy.len() {
            if normalizing_sum > 0. {
                self.strategy[i] /= normalizing_sum;
            } else {
                self.strategy[i] = 1. / self.strategy.len() as f32;
            }
        }
        &self.strategy
    }
    pub fn get_average_strategy(&self) -> Vec<f32> {
        let normalizing_sum: f32 = self.strategy_sum.iter().sum();
        if normalizing_sum > 0. {
            self.strategy_sum
                .iter()
                .map(|s| s / normalizing_sum)
                .collect()
        } else {
            vec![1. / self.strategy.len() as f32; self.strategy.len()]
        }
    }

    pub fn get_random_action(&mut self) -> usize {
        let s = self.get_strategy();
        let dist = WeightedIndex::new(s).unwrap();
        for i in 0..self.strategy.len() {
            self.strategy_sum[i] += self.strategy[i];
        }
        dist.sample(&mut rand::thread_rng())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TipoEstrategia {
    CuatroManos = 0,
    TresManos1vs2 = 1,
    TresManos2vs1 = 2,
    DosManos = 3,
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
                [parejas[0][0].to_string(), parejas[1][0].to_string()],
            )
        } else if parejas[0].len() == 1 && parejas[1].len() == 2 {
            (
                TipoEstrategia::TresManos1vs2,
                [
                    parejas[0][0].to_string(),
                    parejas[1][0].to_string() + "," + &parejas[1][1].to_string(),
                ],
            )
        } else {
            (
                TipoEstrategia::TresManos2vs1,
                [
                    parejas[0][0].to_string() + "," + &parejas[0][1].to_string(),
                    parejas[1][0].to_string(),
                ],
            )
        }
    }
}

#[derive(Debug)]
pub struct PartidaLance {
    manos: Vec<Mano>,
    manos_normalizadas: [String; 2],
    tipo_estrategia: TipoEstrategia,
    lance: Lance,
    tantos: [u8; 2],
}

impl PartidaLance {
    pub fn new_random(lance: Lance, tantos: [u8; 2]) -> Self {
        let mut manos;
        loop {
            let b = Self::crear_baraja();
            manos = Self::repartir_manos(b);
            if lance.se_juega(&manos) {
                break;
            }
        }
        let (tipo_estrategia, manos_normalizadas) = TipoEstrategia::normalizar_mano(&manos, &lance);
        Self {
            manos,
            lance,
            tantos,
            manos_normalizadas,
            tipo_estrategia,
        }
    }

    fn crear_baraja() -> Baraja {
        let mut b = Baraja::new();
        for _ in 0..8 {
            b.insertar(mus::Carta::As);
            b.insertar(mus::Carta::Rey);
        }
        for _ in 0..4 {
            b.insertar(mus::Carta::Caballo);
            // b.insertar(mus::Carta::Sota);
            // b.insertar(mus::Carta::Siete);
            // b.insertar(mus::Carta::Seis);
            // b.insertar(mus::Carta::Cinco);
            b.insertar(mus::Carta::Cuatro);
        }
        b.barajar();
        b
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

#[derive(Debug)]
pub struct Cfr {
    history: Vec<Accion>,
    nodos: HashMap<String, Node>,
}

impl Cfr {
    fn info_set_str(&self, player: usize, mano1: &str, history: &[Accion]) -> String {
        let mut output = String::with_capacity(11 + history.len() + 1);
        output.push(if player == 0 { '0' } else { '1' });
        output.push(',');
        output.push_str(mano1);
        output.push(',');
        for i in history.iter() {
            output.push_str(&i.to_string());
        }
        output
    }

    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            nodos: HashMap::new(),
        }
    }

    pub fn nodes(&self) -> &HashMap<String, Node> {
        &self.nodos
    }

    pub fn external_cfr(
        &mut self,
        partida_lance: &PartidaLance,
        n: &ActionNode<usize, Accion>,
        player: usize,
    ) -> f32 {
        match n {
            ActionNode::NonTerminal(p, children) => {
                let info_set_str =
                    self.info_set_str(*p, &partida_lance.manos_normalizadas[*p], &self.history);
                self.nodos
                    .entry(info_set_str.clone())
                    .or_insert(Node::new(children.len()));
                if *p == player {
                    let mut util = vec![0.; children.len()];
                    for (i, (a, child)) in children.iter().enumerate() {
                        self.history.push(*a);
                        util[i] = self.external_cfr(partida_lance, child, player);
                        self.history.pop();
                    }
                    let nodo = self.nodos.get_mut(&info_set_str).unwrap();
                    let strategy = nodo.get_strategy();
                    let mut node_util = 0.;

                    util.iter().enumerate().for_each(|(i, u)| {
                        node_util += strategy[i] * u;
                    });
                    util.iter().enumerate().for_each(|(i, u)| {
                        let regret = u - node_util;
                        nodo.regret_sum[i] += regret;
                    });
                    node_util
                } else {
                    let s = self
                        .nodos
                        .get_mut(&info_set_str)
                        .unwrap()
                        .get_random_action();
                    let accion = children.get(s).unwrap();

                    self.history.push(accion.0);
                    let util = self.external_cfr(partida_lance, &accion.1, player);
                    self.history.pop();
                    util
                }
            }
            ActionNode::Terminal => {
                let mut estado_lance =
                    EstadoLance::new(partida_lance.lance.apuesta_minima(), 40, 0);
                self.history.iter().for_each(|&a| {
                    let _ = estado_lance.actuar(a);
                });
                estado_lance.resolver_lance(&partida_lance.manos, &partida_lance.lance);
                let mut tantos: [u8; 2] = partida_lance.tantos;

                let ganador = estado_lance.ganador().unwrap();
                let apuesta = estado_lance.tantos_apostados();
                match apuesta {
                    Apuesta::Tantos(t) => tantos[ganador] += t,
                    Apuesta::Ordago => tantos[ganador] = 40,
                }
                if tantos[ganador] < 40 {
                    tantos[ganador] += partida_lance.lance.bonus();
                }

                // if tantos[ganador] < 40 {
                //     tantos[ganador] += Lance::Pares.tantos_mano(&self.manos[ganador]) as i8;
                // }
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
        }
    }
}

impl Default for Cfr {
    fn default() -> Self {
        Self::new()
    }
}

pub struct BancoEstrategias {
    grande: Vec<Cfr>,
    chica: Vec<Cfr>,
    pares: Vec<Cfr>,
    juego: Vec<Cfr>,
    punto: Vec<Cfr>,
}

impl BancoEstrategias {
    pub fn new() -> Self {
        Self {
            grande: vec![Cfr::new()],
            chica: vec![Cfr::new()],
            pares: vec![Cfr::new(), Cfr::new(), Cfr::new(), Cfr::new()],
            juego: vec![Cfr::new(), Cfr::new(), Cfr::new(), Cfr::new()],
            punto: vec![Cfr::new()],
        }
    }

    pub fn estrategia_lance(&self, l: Lance, t: TipoEstrategia) -> &Cfr {
        match l {
            Lance::Grande => &self.grande[0],
            Lance::Chica => &self.chica[0],
            Lance::Pares => &self.pares[t as usize],
            Lance::Punto => &self.punto[0],
            Lance::Juego => &self.juego[t as usize],
        }
    }
    pub fn estrategia_lance_mut(&mut self, l: Lance, t: TipoEstrategia) -> &mut Cfr {
        match l {
            Lance::Grande => &mut self.grande[0],
            Lance::Chica => &mut self.chica[0],
            Lance::Pares => &mut self.pares[t as usize],
            Lance::Punto => &mut self.punto[0],
            Lance::Juego => &mut self.juego[t as usize],
        }
    }

    fn export_estrategia(&self, l: Lance, t: TipoEstrategia) -> std::io::Result<()> {
        let file_name = format!("{:?}_{:?}.csv", l, t);
        let mut file = File::create(file_name)?;
        let c = self.estrategia_lance(l, t);

        let mut v: Vec<(String, Node)> = c
            .nodes()
            .iter()
            .map(|(s, n)| (s.clone(), n.clone()))
            .collect();
        v.sort_by(|x, y| x.0.cmp(&y.0));
        for (k, n) in v {
            writeln!(
                file,
                "{},{}",
                k,
                n.get_average_strategy()
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            )?;
        }
        Ok(())
    }

    pub fn export(&self) -> std::io::Result<()> {
        self.export_estrategia(Lance::Grande, TipoEstrategia::CuatroManos)?;
        self.export_estrategia(Lance::Chica, TipoEstrategia::CuatroManos)?;
        self.export_estrategia(Lance::Punto, TipoEstrategia::CuatroManos)?;
        self.export_estrategia(Lance::Pares, TipoEstrategia::CuatroManos)?;
        self.export_estrategia(Lance::Pares, TipoEstrategia::DosManos)?;
        self.export_estrategia(Lance::Pares, TipoEstrategia::TresManos1vs2)?;
        self.export_estrategia(Lance::Pares, TipoEstrategia::TresManos2vs1)?;
        self.export_estrategia(Lance::Juego, TipoEstrategia::CuatroManos)?;
        self.export_estrategia(Lance::Juego, TipoEstrategia::DosManos)?;
        self.export_estrategia(Lance::Juego, TipoEstrategia::TresManos1vs2)?;
        self.export_estrategia(Lance::Juego, TipoEstrategia::TresManos2vs1)?;
        Ok(())
    }
}

impl Default for BancoEstrategias {
    fn default() -> Self {
        Self::new()
    }
}
