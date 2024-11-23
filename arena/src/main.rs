use core::panic;
use std::{io, path::PathBuf};

use clap::{command, Parser, ValueEnum};
use musolver::{
    mus::{
        arena::{
            ActionRecorder, AgenteAleatorio, AgenteCli, AgenteMusolver, Kibitzer, MusAction,
            MusArena,
        },
        Juego, Lance, Mano, PartidaMus,
    },
    solver::{BancoEstrategias, LanceGame, SolverError, Strategy, StrategyConfig},
};

pub struct KibitzerCli {
    manos: Vec<Mano>,
    marcador: [usize; 2],
    cli_player: usize,
    jugador_mano: usize,
    lance_actual: Option<Lance>,
    nombres_jugadores: Vec<String>,
}

impl KibitzerCli {
    pub fn new(nombres_jugadores: Vec<String>, cli_player: usize) -> Self {
        Self {
            manos: vec![],
            marcador: [0, 0],
            cli_player,
            jugador_mano: 0,
            lance_actual: None,
            nombres_jugadores,
        }
    }

    fn hand_str(lance: &Lance, m: &Mano, hidden: bool) -> String {
        let hay_jugada = match lance {
            Lance::Grande | Lance::Chica | Lance::Punto => false,
            Lance::Pares => m.pares().is_some(),
            Lance::Juego => m.juego().is_some(),
        };
        let ayuda_valor = match lance {
            Lance::Juego => m
                .juego()
                .map(|j| match j {
                    Juego::Resto(v) => format!("({v})"),
                    Juego::Treintaydos => "(32)".to_string(),
                    Juego::Treintayuna => "(31)".to_string(),
                })
                .unwrap_or_default(),
            Lance::Punto => format!("({})", m.valor_puntos()),
            _ => "".to_string(),
        };
        let suffix = if hay_jugada {
            "*".to_owned()
        } else {
            "".to_owned()
        };
        if hidden {
            format!("XXXX {suffix}")
        } else {
            format!("{m} {ayuda_valor} {suffix}")
        }
    }
}

impl Kibitzer for KibitzerCli {
    fn record(&mut self, partida_mus: &PartidaMus, action: MusAction) {
        match &action {
            MusAction::GameStart(dealer_id) => {
                self.lance_actual = None;
                self.jugador_mano = *dealer_id;
                self.manos.clear();
                println!();
                println!();
                println!("🥊🥊🥊 Game starts! Fight! 🥊🥊🥊");
                println!("Marcador: {}-{}", self.marcador[0], self.marcador[1]);
                println!();
                println!();
            }
            MusAction::DealHand(player_id, m) => {
                let lance = partida_mus.lance_actual().unwrap();
                let hand_str = KibitzerCli::hand_str(&lance, m, *player_id != self.cli_player);
                let es_mano = if *player_id == self.jugador_mano {
                    "(M)"
                } else {
                    ""
                };
                println!(
                    "{} {es_mano}: {hand_str}",
                    self.nombres_jugadores[*player_id]
                );
                self.manos.push(m.clone());
            }
            MusAction::PlayerAction(player_id, accion) => {
                if *player_id != self.cli_player {
                    println!(
                        "❗❗❗{} ha actuado: {:?}",
                        self.nombres_jugadores[*player_id], accion
                    );
                }
            }
            MusAction::Payoff(pareja_id, tantos) => {
                if *tantos > 0 {
                    if *pareja_id == self.cli_player % 2 {
                        println!();
                        println!("¡¡¡¡HAS GANADO {tantos} tantos!!!! 🚀🚀🚀");
                        println!();
                    } else {
                        println!("Pareja rival ha ganado {tantos} tantos con manos.",);
                    }
                    for mano in &self.manos {
                        println!(
                            "{}",
                            KibitzerCli::hand_str(&self.lance_actual.unwrap(), mano, false)
                        );
                    }
                }
                self.marcador[*pareja_id] += *tantos as usize;
            }
            MusAction::LanceStart(lance) => self.lance_actual = Some(*lance),
        }
    }
}
fn show_strategy_data(strategy: &StrategyConfig) {
    println!(
        "\tLance: {}",
        strategy
            .game_config
            .lance
            .map_or_else(|| "Partida completa".to_string(), |v| format!("{:?}", v))
    );
    println!("\tJuego abstracto: {}", strategy.game_config.abstract_game);
    println!("\tIteraciones:{:?}", strategy.trainer_config.iterations);
    println!("\tMétodo de cálculo: {:?}", strategy.trainer_config.method);
    println!();
}

fn pick_musolver_strategy() -> String {
    let estrategias = BancoEstrategias::find(PathBuf::from("output").as_path());
    for (idx, s) in estrategias.iter().enumerate() {
        println!("{idx}: {}", s.0);
        show_strategy_data(&s.1);
    }
    let mut input = String::new();

    let strategy = loop {
        println!("Elige una estrategia [0-{}]:", estrategias.len() - 1);
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");
        let num = input.trim().parse::<usize>();
        match num {
            Ok(n) => {
                if n < estrategias.len() {
                    break estrategias[n].0.to_owned();
                } else {
                    println!("Opción no válida.");
                    input.clear();
                }
            }
            Err(_) => {
                println!("Opción no válida.");
                input.clear();
            }
        }
    };
    strategy
}

#[derive(Debug, ValueEnum, Clone, PartialEq)]
enum AgentType {
    Cli,
    Random,
    Musolver,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Ruta al fihero JSON que contiene la estrategia a utilizar.
    #[arg(short, long)]
    strategy_path: Option<String>,

    #[arg(short, long, num_args = 4, required = true, value_enum)]
    agents: Vec<AgentType>,
}

fn main() {
    let args = Args::parse();

    let strategy: Option<Strategy<LanceGame>> = if args
        .agents
        .iter()
        .any(|agent| *agent == AgentType::Musolver)
    {
        let strategy_path = PathBuf::from(match args.strategy_path {
            Some(path) => path,
            None => pick_musolver_strategy(),
        });

        let strategy = match Strategy::from_file(strategy_path.as_path()) {
            Ok(s) => s,
            Err(SolverError::InvalidStrategyPath(err, path)) => {
                panic!("Cannot open strategy file: {}. ({})", path, err)
            }
            Err(SolverError::StrategyParseJsonError(err)) => {
                panic!("Cannot parse strategy file: {}", err)
            }
            Err(err) => {
                panic!("Unexpected error: {}", err)
            }
        };
        println!();
        println!("Cargada la siguiente estrategia:");
        show_strategy_data(&strategy.strategy_config);
        Some(strategy)
    } else {
        None
    };

    let lance = if let Some(s) = &strategy {
        s.strategy_config.game_config.lance
    } else {
        None
    };

    let mut arena = MusArena::new(lance);

    let action_recorder = ActionRecorder::new();

    let agente_aleatorio = AgenteAleatorio::new(action_recorder.history());
    let agente_cli = AgenteCli::new(action_recorder.history());

    let mut cli_client = 0;
    let mut nombres_jugadores = vec![];
    for (i, agent) in args.agents.iter().enumerate() {
        match agent {
            AgentType::Cli => {
                arena.agents.push(Box::new(agente_cli.clone()));
                cli_client = i;
                nombres_jugadores.push(format!("Hero#{i}"));
            }
            AgentType::Random => {
                arena.agents.push(Box::new(agente_aleatorio.clone()));
                nombres_jugadores.push(format!("Random#{i}"));
            }
            AgentType::Musolver => {
                if let Some(s) = &strategy {
                    let agente_musolver =
                        AgenteMusolver::new(s.to_owned(), action_recorder.history());
                    arena.agents.push(Box::new(agente_musolver.clone()))
                } else {
                    panic!("Cannot load musolver: strategy not available.");
                }
                nombres_jugadores.push(format!("Musolver#{i}"));
            }
        }
    }

    let kibitzer_cli = KibitzerCli::new(nombres_jugadores, cli_client);
    arena.kibitzers.push(Box::new(action_recorder));
    arena.kibitzers.push(Box::new(kibitzer_cli));

    loop {
        arena.start();
    }
}
