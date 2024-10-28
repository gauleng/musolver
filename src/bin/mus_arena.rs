use core::panic;
use std::{io, path::PathBuf};

use clap::{command, Parser, ValueEnum};
use musolver::{
    mus::arena::{
        ActionRecorder, AgenteAleatorio, AgenteCli, AgenteMusolver, KibitzerCli, MusArena,
    },
    solver::{BancoEstrategias, SolverError, Strategy, StrategyConfig},
};

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

    #[arg(long, value_enum)]
    agent1: AgentType,

    #[arg(long, value_enum)]
    agent2: AgentType,
}

fn main() {
    let args = Args::parse();

    let strategy: Option<Strategy> =
        if args.agent1 == AgentType::Musolver || args.agent2 == AgentType::Musolver {
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
    match args.agent1 {
        AgentType::Cli => {
            arena.agents.push(Box::new(agente_cli.clone()));
            cli_client = 0;
        }
        AgentType::Random => arena.agents.push(Box::new(agente_aleatorio.clone())),
        AgentType::Musolver => {
            if let Some(s) = &strategy {
                let agente_musolver = AgenteMusolver::new(s.to_owned(), action_recorder.history());
                arena.agents.push(Box::new(agente_musolver.clone()))
            } else {
                panic!("Cannot load musolver: strategy not available.");
            }
        }
    }

    match args.agent2 {
        AgentType::Cli => {
            arena.agents.push(Box::new(agente_cli.clone()));
            cli_client = 1;
        }
        AgentType::Random => arena.agents.push(Box::new(agente_aleatorio.clone())),
        AgentType::Musolver => {
            if let Some(s) = &strategy {
                let agente_musolver = AgenteMusolver::new(s.to_owned(), action_recorder.history());
                arena.agents.push(Box::new(agente_musolver.clone()))
            } else {
                panic!("Cannot load musolver: strategy not available.");
            }
        }
    }
    let kibitzer_cli = KibitzerCli::new(cli_client);
    arena.kibitzers.push(Box::new(action_recorder));
    arena.kibitzers.push(Box::new(kibitzer_cli));

    loop {
        arena.start();
    }
}
