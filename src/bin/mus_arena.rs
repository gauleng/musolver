use std::{cell::RefCell, io, path::PathBuf, rc::Rc};

use musolver::{
    mus::{Accion, Baraja, Lance, Mano, PartidaMus},
    solver::{BancoEstrategias, TipoEstrategia},
    ActionNode,
};
use rand::{distributions::WeightedIndex, prelude::Distribution};

trait Agent {
    fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion;
}

#[derive(Clone, Debug)]
enum MusAction {
    /// Game starts.
    GameStart(usize),
    DealHand(usize, Mano),
    LanceStart(Lance),
    PlayerAction(usize, Accion),
    Payoff(usize, u8),
}

trait Kibitzer {
    fn record(&mut self, partida_mus: &PartidaMus, action: MusAction);
}

struct AgenteCli {
    history: Rc<RefCell<Vec<Accion>>>,
    action_tree: ActionNode<usize, Accion>,
}

impl AgenteCli {
    pub fn new(action_tree: ActionNode<usize, Accion>, history: Rc<RefCell<Vec<Accion>>>) -> Self {
        Self {
            history,
            action_tree,
        }
    }
}

impl Agent for AgenteCli {
    fn actuar(&mut self, _partida_mus: &PartidaMus) -> Accion {
        println!("Elija una acción:");
        let node = self.action_tree.search_action_node(&self.history.borrow());
        if let ActionNode::NonTerminal(_, next_actions) = node {
            let acciones: Vec<Accion> = next_actions.iter().map(|c| c.0).collect();
            acciones
                .iter()
                .enumerate()
                .for_each(|(i, a)| println!("{i}: {:?}", a));
            let mut input = String::new();
            loop {
                io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");
                let num = input.trim().parse::<usize>();
                match num {
                    Ok(n) => {
                        if n < acciones.len() {
                            return acciones[n];
                        } else {
                            println!("Opción no válida.");
                        }
                    }
                    Err(_) => {
                        println!("Opción no válida.");
                        input.clear();
                    }
                }
            }
        }
        Accion::Paso
    }
}

struct ActionRecorder {
    history: Rc<RefCell<Vec<Accion>>>,
}
impl ActionRecorder {
    fn new() -> Self {
        Self {
            history: Rc::new(RefCell::new(vec![])),
        }
    }
}

impl Kibitzer for ActionRecorder {
    fn record(&mut self, _partida_mus: &PartidaMus, action: MusAction) {
        match &action {
            MusAction::GameStart(_) => self.history.borrow_mut().clear(),
            MusAction::PlayerAction(_, accion) => self.history.borrow_mut().push(*accion),
            _ => {}
        }
    }
}

struct KibitzerCli {
    manos: Vec<Mano>,
    marcador: [usize; 2],
    cli_player: usize,
    pareja_mano: usize,
}

impl KibitzerCli {
    fn new(cli_player: usize) -> Self {
        Self {
            manos: vec![],
            marcador: [0, 0],
            cli_player,
            pareja_mano: 0,
        }
    }
}

impl Kibitzer for KibitzerCli {
    fn record(&mut self, partida_mus: &PartidaMus, action: MusAction) {
        match &action {
            MusAction::GameStart(p) => {
                self.pareja_mano = *p;
                self.manos.clear();
                println!();
                println!();
                println!("Game starts!");
                println!("Marcador: {}-{}", self.marcador[0], self.marcador[1]);
                println!();
                println!();
            }
            MusAction::DealHand(p, m) => {
                let hay_jugada = if let Some(lance) = partida_mus.lance_actual() {
                    match lance {
                        Lance::Grande | Lance::Chica | Lance::Punto => false,
                        Lance::Pares => m.pares().is_some(),
                        Lance::Juego => m.juego().is_some(),
                    }
                } else {
                    false
                };
                let valor = m
                    .juego()
                    .map(|j| match j {
                        musolver::mus::Juego::Resto(v) => format!("({v})"),
                        musolver::mus::Juego::Treintaydos => "(32)".to_string(),
                        musolver::mus::Juego::Treintayuna => "(31)".to_string(),
                    })
                    .unwrap_or_default();
                let suffix = if hay_jugada {
                    "*".to_owned()
                } else {
                    "".to_owned()
                };
                if self.pareja_mano == self.cli_player && p % 2 == 0
                    || self.pareja_mano != self.cli_player && p % 2 == 1
                {
                    println!("Mano jugador {p}: {m} {valor} {suffix}");
                } else {
                    println!("Mano jugador {p}: XXXX {suffix}");
                }
                self.manos.push(m.clone());
            }
            MusAction::PlayerAction(p, accion) => {
                println!("Pareja {p} ha actuado: {:?}", accion);
            }
            MusAction::Payoff(p, t) => {
                let pareja = if *p == self.pareja_mano { 0 } else { 1 };
                println!(
                    "Pareja {p} ha ganado {t} tantos con manos: {} {}",
                    self.manos[pareja],
                    self.manos[pareja + 2]
                );
                self.marcador[*p] += *t as usize;
            }
            MusAction::LanceStart(lance) => println!("Lance: {:?}", lance),
        }
    }
}

struct AgenteMusolver {
    banco: BancoEstrategias,
    action_tree: ActionNode<usize, Accion>,
    history: Rc<RefCell<Vec<Accion>>>,
}

impl AgenteMusolver {
    pub fn new(
        banco: BancoEstrategias,
        action_tree: ActionNode<usize, Accion>,
        history: Rc<RefCell<Vec<Accion>>>,
    ) -> Self {
        Self {
            action_tree,
            banco,
            history,
        }
    }
}

impl Agent for AgenteMusolver {
    fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion {
        let lance = partida_mus.lance_actual().unwrap();
        let turno_inicial = lance.turno_inicial(partida_mus.manos());
        let mut turno = partida_mus.turno().unwrap();
        if turno_inicial == 1 {
            turno = 1 - turno;
        }
        let (tipo_estrategia, manos_normalizadas) =
            TipoEstrategia::normalizar_mano(partida_mus.manos(), &lance);
        let manos_normalizadas_str = manos_normalizadas.to_abstract_string_array(&lance);
        let h: Vec<String> = self
            .history
            .borrow()
            .iter()
            .map(|a| a.to_string())
            .collect();
        let info_set = format!(
            "{},{},{}",
            tipo_estrategia,
            manos_normalizadas_str[turno],
            h.join("")
        );
        let cfr = self.banco.estrategia_lance(lance);
        let n = cfr.nodes().get(&info_set).unwrap();
        // println!("{info_set} {:?}", n.strategy());
        let dist = WeightedIndex::new(n.strategy()).unwrap();
        let a = dist.sample(&mut rand::thread_rng());
        let node = self.action_tree.search_action_node(&self.history.borrow());
        if let ActionNode::NonTerminal(_, children) = node {
            children[a].0
        } else {
            Accion::Paso
        }
    }
}

struct MusArena {
    agents: Vec<Box<dyn Agent>>,
    kibitzers: Vec<Box<dyn Kibitzer>>,
    partida_mus: PartidaMus,
    order: [usize; 2],
}

impl MusArena {
    pub fn new() -> Self {
        MusArena {
            agents: vec![],
            kibitzers: vec![],
            partida_mus: MusArena::new_partida_lance(),
            order: [0, 1],
        }
    }

    fn new_partida_lance() -> PartidaMus {
        let mut baraja = Baraja::baraja_mus();
        loop {
            baraja.barajar();
            let manos = baraja.repartir_manos();
            let posible_partida_mus = PartidaMus::new_partida_lance(Lance::Juego, manos, [0, 0]);
            if let Some(partida_mus) = posible_partida_mus {
                return partida_mus;
            }
        }
    }

    fn record_action(&mut self, a: MusAction) {
        self.kibitzers
            .iter_mut()
            .for_each(|k| k.record(&self.partida_mus, a.clone()));
    }

    pub fn start(&mut self) {
        self.partida_mus = MusArena::new_partida_lance();
        self.order.swap(0, 1);
        self.record_action(MusAction::GameStart(self.order[0]));
        let manos = self.partida_mus.manos().clone();
        for (i, m) in manos.iter().enumerate() {
            self.record_action(MusAction::DealHand(i, m.clone()));
        }
        self.record_action(MusAction::LanceStart(
            self.partida_mus.lance_actual().unwrap(),
        ));
        while let Some(turno) = self.partida_mus.turno() {
            let accion = self.agents[self.order[turno]].actuar(&self.partida_mus);
            self.record_action(MusAction::PlayerAction(self.order[turno], accion));
            let _ = self.partida_mus.actuar(accion);
        }
        let tantos = *self.partida_mus.tantos();
        self.record_action(MusAction::Payoff(self.order[0], tantos[0]));
        self.record_action(MusAction::Payoff(self.order[1], tantos[1]));
    }
}

fn main() {
    let estrategia_path = PathBuf::from("output/2024-09-27 22:56/");
    let banco = BancoEstrategias::new();
    let trainer_config = banco
        .load_estrategia(estrategia_path.as_path(), Lance::Juego)
        .expect("Error cargando estrategia");

    let mut arena = MusArena::new();

    let kibitzer_cli = KibitzerCli::new(1);
    let action_recorder = ActionRecorder::new();

    let agente_cli = AgenteCli::new(
        trainer_config.action_tree.clone(),
        action_recorder.history.clone(),
    );
    let agente_musolver = AgenteMusolver::new(
        banco,
        trainer_config.action_tree.clone(),
        action_recorder.history.clone(),
    );

    arena.kibitzers.push(Box::new(action_recorder));
    arena.kibitzers.push(Box::new(kibitzer_cli));

    arena.agents.push(Box::new(agente_musolver));
    arena.agents.push(Box::new(agente_cli));

    loop {
        arena.start();
    }
}
