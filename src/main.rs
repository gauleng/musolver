use std::path::{Path, PathBuf};

use chrono::Utc;
use musolver::{
    Cfr, CfrMethod,
    mus::Lance,
    solver::{
        GameConfig, GameType, LanceGame, MusGame, MusGameTwoHands, MusGameTwoPlayers, SolverError,
        Strategy, Trainer, TrainerConfig,
    },
};

use clap::{Parser, ValueEnum};

#[derive(Parser, Clone, Debug, ValueEnum)]
enum MusVariant {
    TwoHands = 0,
    TwoPlayers = 1,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Número de iteraciones de CFR.
    #[arg(short, long)]
    iter: usize,

    /// Lance a simular. Si no se pasa este parámetro se simula la partida completa.
    #[arg(short, long, value_enum)]
    lance: Option<Lance>,

    /// Marcador inicial de tantos al comienzo de la partida. Deben separarse los números mediante
    /// dos puntos. Por ejemplo, 25:14.
    #[arg(short, long, value_parser = parse_tantos)]
    tantos: Option<[u8; 2]>,

    /// Variante de CFR a utilizar. Por defecto: chance-sampling
    #[arg(short, long, value_enum)]
    method: Option<CfrMethod>,

    /// Ruta donde se desean guardar las estrategias generadas. Por defecto: output/
    #[arg(short, long)]
    output: Option<String>,

    /// Si se activa esta opción se calcula la estrategia para una versión simplificada del mus. En
    /// grande y chica solo se tienen en cuenta las dos cartas más significativas. En pares, juego
    /// y punto el valor de la jugada.
    #[arg(long)]
    abstract_game: bool,

    /// Se calcula la estrategia asumiendo que cada pareja conoce las dos manos y solo hay una
    /// acción por pareja.
    #[arg(long)]
    variant: Option<MusVariant>,
}

fn parse_tantos(s: &str) -> Result<[u8; 2], String> {
    let t: Vec<&str> = s.split(":").collect();
    if t.len() != 2 {
        Err(format!(
            "Formato de los tantos incorecto ({s}). Deben indicarse separados por dos puntos, por ejemplo 5:23."
        ))
    } else {
        let tantos1: u8 = t[0]
            .parse()
            .map_err(|_| format!("{} no es un número.", t[0]))?;
        let tantos2: u8 = t[1]
            .parse()
            .map_err(|_| format!("{} no es un número.", t[1]))?;
        Ok([tantos1, tantos2])
    }
}

fn main() {
    let args = Args::parse();

    let tantos = args.tantos.unwrap_or_default();
    let trainer = Trainer {};
    let method = args.method.unwrap_or(CfrMethod::ChanceSampling);
    let mut output_path = PathBuf::from(args.output.unwrap_or_else(|| "output/".to_string()));

    let trainer_config = TrainerConfig {
        iterations: args.iter,
        method,
    };
    let game_config = GameConfig {
        abstract_game: args.abstract_game,
        game_type: match (args.lance, args.variant) {
            (Some(lance), _) => GameType::LanceGame(lance),
            (None, None) => GameType::MusGame,
            (None, Some(MusVariant::TwoHands)) => GameType::MusGameTwoHands,
            (None, Some(MusVariant::TwoPlayers)) => GameType::MusGameTwoPlayers,
        },
    };
    println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    println!(
        "Simulando: {}",
        match game_config.game_type {
            GameType::LanceGame(lance) => format!("{lance:?}"),
            GameType::MusGame => "Partida completa".into(),
            GameType::LanceGameTwoHands(_) => todo!(),
            GameType::MusGameTwoHands => "Partida completa (two hands)".into(),
            GameType::MusGameTwoPlayers => "Partida completa (two players)".into(),
        }
    );
    println!("Tantos iniciales: {}:{}", tantos[0], tantos[1]);

    let mut cfr = Cfr::new();
    match game_config.game_type {
        GameType::LanceGame(lance) => {
            let mut lance_game = LanceGame::new(lance, tantos, game_config.abstract_game);
            trainer.train(&mut cfr, &mut lance_game, &trainer_config);
        }
        GameType::MusGame => {
            let mut mus_game = MusGame::new(tantos, game_config.abstract_game);
            trainer.train(&mut cfr, &mut mus_game, &trainer_config);
        }
        GameType::LanceGameTwoHands(_) => todo!(),
        GameType::MusGameTwoHands => {
            let mut mus_game = MusGameTwoHands::new(tantos, game_config.abstract_game);
            trainer.train(&mut cfr, &mut mus_game, &trainer_config);
        }
        GameType::MusGameTwoPlayers => {
            let mut mus_game = MusGameTwoPlayers::new(tantos, game_config.abstract_game);
            trainer.train(&mut cfr, &mut mus_game, &trainer_config);
        }
    }
    let curr_time = Utc::now();
    output_path.push(format!("{}", curr_time.format("%Y-%m-%d %H%M")));
    println!("Exportando estrategias a {output_path:?}...");
    export_cfr(&output_path, &cfr, &trainer_config, &game_config)
        .expect("Error exportando estrategias.");
}

pub fn export_cfr(
    path: &Path,
    cfr: &Cfr,
    trainer_config: &TrainerConfig,
    game_config: &GameConfig,
) -> Result<(), SolverError> {
    let mut estrategia_path = PathBuf::from(path);
    estrategia_path.set_extension("json");
    let strategy = Strategy::new(cfr, trainer_config, game_config);
    strategy.to_file(estrategia_path)
}
