use core::panic;
use std::path::PathBuf;

use clap::{command, Parser, ValueEnum};
use musolver::{
    mus::arena::{
        ActionRecorder, AgenteAleatorio, AgenteCli, AgenteMusolver, KibitzerCli, MusArena,
    },
    solver::{SolverError, Strategy},
};

fn show_strategy_data(strategy: &Strategy) {
    println!("Estrategia cargada:");
    println!("Lance: {:?}", strategy.strategy_config.game_config.lance);
    println!(
        "Juego abstracto: {}",
        strategy.strategy_config.game_config.abstract_game
    );
    println!(
        "Iteraciones:{:?}",
        strategy.strategy_config.trainer_config.iterations
    );
    println!(
        "Método de cálculo: {:?}",
        strategy.strategy_config.trainer_config.method
    );
    println!();
}

#[derive(Debug, ValueEnum, Clone)]
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
    strategy_path: String,

    #[arg(long, value_enum)]
    agent1: AgentType,

    #[arg(long, value_enum)]
    agent2: AgentType,
}

fn main() {
    let args = Args::parse();

    let estrategia_path = PathBuf::from(args.strategy_path);
    let strategy = match Strategy::from_file(estrategia_path.as_path()) {
        Ok(s) => s,
        Err(SolverError::InvalidStrategyPath(err, path)) => {
            panic!("Cannot open strategy file: {}. ({})", path, err)
        }
        Err(SolverError::StrategyParseJsonError(err)) => {
            panic!("Cannot parse strategy file: {}", err)
        }
        Err(SolverError::NoCreateFolderPermission(err, _)) => {
            panic!("Cannot create folder {}", err)
        }
    };
    show_strategy_data(&strategy);

    let mut arena = MusArena::new(strategy.strategy_config.game_config.lance);

    let action_recorder = ActionRecorder::new();

    let agente_aleatorio = AgenteAleatorio::new(
        strategy.strategy_config.trainer_config.action_tree.clone(),
        action_recorder.history(),
    );
    let agente_cli = AgenteCli::new(
        strategy.strategy_config.trainer_config.action_tree.clone(),
        action_recorder.history(),
    );
    let agente_musolver = AgenteMusolver::new(strategy, action_recorder.history());

    arena.kibitzers.push(Box::new(action_recorder));

    let mut cli_client = 0;
    match args.agent1 {
        AgentType::Cli => {
            arena.agents.push(Box::new(agente_cli.clone()));
            cli_client = 0;
        }
        AgentType::Random => arena.agents.push(Box::new(agente_aleatorio.clone())),
        AgentType::Musolver => arena.agents.push(Box::new(agente_musolver.clone())),
    }

    match args.agent2 {
        AgentType::Cli => {
            arena.agents.push(Box::new(agente_cli.clone()));
            cli_client = 1;
        }
        AgentType::Random => arena.agents.push(Box::new(agente_aleatorio.clone())),
        AgentType::Musolver => arena.agents.push(Box::new(agente_musolver.clone())),
    }
    let kibitzer_cli = KibitzerCli::new(cli_client);
    arena.kibitzers.push(Box::new(kibitzer_cli));

    loop {
        arena.start();
    }
}
