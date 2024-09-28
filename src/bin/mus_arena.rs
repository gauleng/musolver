use std::{cell::RefCell, io, path::PathBuf, rc::Rc};

use clap::{command, Parser};
use musolver::{
    mus::{Accion, Baraja, Lance, Mano, PartidaMus},
    solver::{LanceGame, Strategy},
    ActionNode, Game, Node,
};
use rand::{distributions::WeightedIndex, prelude::Distribution, Rng};

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
                println!("🥊🥊🥊 Game starts! Fight! 🥊🥊🥊");
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
                let ayuda_valor = m
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
                    println!("Mano jugador {p}: {m} {ayuda_valor} {suffix}");
                } else {
                    println!("Mano jugador {p}: XXXX {suffix}");
                }
                self.manos.push(m.clone());
            }
            MusAction::PlayerAction(p, accion) => {
                if *p != self.cli_player {
                    println!("❗❗❗Pareja {p} ha actuado: {:?}", accion);
                }
            }
            MusAction::Payoff(p, t) => {
                if *t > 0 {
                    let pareja = if *p == self.pareja_mano { 0 } else { 1 };
                    if *p == self.cli_player {
                        println!();
                        println!("¡¡¡¡HAS GANADO {t} tantos!!!! 🚀🚀🚀");
                        println!();
                        println!(
                            "Manos del rival: {} {}",
                            self.manos[1 - pareja],
                            self.manos[3 - pareja]
                        );
                    } else {
                        println!(
                            "Pareja {p} ha ganado {t} tantos con manos: {} {}",
                            self.manos[pareja],
                            self.manos[pareja + 2]
                        );
                    }
                }
                self.marcador[*p] += *t as usize;
            }
            MusAction::LanceStart(lance) => println!("Lance: {:?}", lance),
        }
    }
}

struct AgenteAleatorio {
    history: Rc<RefCell<Vec<Accion>>>,
    action_tree: ActionNode<usize, Accion>,
}

impl AgenteAleatorio {
    pub fn new(action_tree: ActionNode<usize, Accion>, history: Rc<RefCell<Vec<Accion>>>) -> Self {
        Self {
            history,
            action_tree,
        }
    }
}

impl Agent for AgenteAleatorio {
    fn actuar(&mut self, _partida_mus: &PartidaMus) -> Accion {
        let next_actions = self
            .action_tree
            .search_action_node(&self.history.borrow())
            .children();
        match next_actions {
            None => {
                println!(
                    "ERROR: La lista de acciones no está en el árbol. {:?}. Se pasa por defecto.",
                    self.history.borrow()
                );
                Accion::Paso
            }
            Some(c) => {
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0..c.len());
                c[idx].0
            }
        }
    }
}

struct AgenteMusolver {
    strategy: Strategy,
    history: Rc<RefCell<Vec<Accion>>>,
}

impl AgenteMusolver {
    pub fn new(strategy: Strategy, history: Rc<RefCell<Vec<Accion>>>) -> Self {
        Self { strategy, history }
    }

    fn accion_aleatoria(&mut self, partida_mus: &PartidaMus, acciones: Vec<Accion>) -> Accion {
        let lance = partida_mus.lance_actual().unwrap();
        let turno_inicial = lance.turno_inicial(partida_mus.manos());
        let mut turno = partida_mus.turno().unwrap();
        if turno_inicial == 1 {
            turno = 1 - turno;
        }
        let info_set = LanceGame::from_partida_mus(
            partida_mus,
            self.strategy.strategy_config.game_config.abstract_game,
        )
        .unwrap()
        .info_set_str(turno, &self.history.borrow());
        let probabilities = match self.strategy.nodes.get(&info_set) {
            None => {
                println!("ERROR: InfoSet no encontrado: {info_set}");
                &Node::new(acciones.len()).strategy().clone()
            }
            Some(n) => n,
        };

        let dist = WeightedIndex::new(probabilities).unwrap();
        let idx = dist.sample(&mut rand::thread_rng());
        acciones[idx]
    }
}

impl Agent for AgenteMusolver {
    fn actuar(&mut self, partida_mus: &PartidaMus) -> Accion {
        let next_actions = self
            .strategy
            .strategy_config
            .trainer_config
            .action_tree
            .search_action_node(&self.history.borrow())
            .children();
        match next_actions {
            None => {
                println!(
                    "ERROR: La lista de acciones no está en el árbol. {:?}. Se pasa por defecto.",
                    self.history.borrow()
                );
                Accion::Paso
            }
            Some(c) => {
                let acciones: Vec<Accion> = c.iter().map(|a| a.0).collect();
                self.accion_aleatoria(partida_mus, acciones)
            }
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Ruta al fihero JSON que contiene la estrategia a utilizar.
    #[arg(short, long)]
    strategy_path: String,
}

fn main() {
    let args = Args::parse();

    let estrategia_path = PathBuf::from(args.strategy_path);
    let strategy =
        Strategy::from_file(estrategia_path.as_path()).expect("Error cargando estrategia");

    let mut arena = MusArena::new();

    let kibitzer_cli = KibitzerCli::new(1);
    let action_recorder = ActionRecorder::new();

    let _agente_aleatorio = AgenteAleatorio::new(
        strategy.strategy_config.trainer_config.action_tree.clone(),
        action_recorder.history.clone(),
    );
    let agente_cli = AgenteCli::new(
        strategy.strategy_config.trainer_config.action_tree.clone(),
        action_recorder.history.clone(),
    );
    let agente_musolver = AgenteMusolver::new(strategy, action_recorder.history.clone());

    arena.kibitzers.push(Box::new(action_recorder));
    arena.kibitzers.push(Box::new(kibitzer_cli));

    arena.agents.push(Box::new(agente_musolver));
    // arena.agents.push(Box::new(agente_aleatorio));
    arena.agents.push(Box::new(agente_cli));

    loop {
        arena.start();
    }
}
