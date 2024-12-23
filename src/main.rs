use std::path::PathBuf;

use chrono::Utc;
use musolver::{
    mus::Lance,
    solver::{BancoEstrategias, GameConfig, LanceGame, Trainer, TrainerConfig},
    CfrMethod,
};

use clap::Parser;

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

    /// Ruta al fichero con el árbol de acciones a considerar en el cálculo del equilibrio. Por
    /// defecto: config/action_tree.json
    #[arg(short, long)]
    action_tree: Option<String>,

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
}

fn parse_tantos(s: &str) -> Result<[u8; 2], String> {
    let t: Vec<&str> = s.split(":").collect();
    if t.len() != 2 {
        Err(format!("Formato de los tantos incorecto ({s}). Deben indicarse separados por dos puntos, por ejemplo 5:23."))
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
    let trainer = args
        .lance
        .map_or_else(|| Trainer::MusTrainer, Trainer::LanceTrainer);
    let method = args.method.unwrap_or(CfrMethod::ChanceSampling);
    let mut output_path = PathBuf::from(args.output.unwrap_or_else(|| "output/".to_string()));

    println!("Musolver 0.1");
    println!(
        "Simulando: {}",
        match trainer {
            Trainer::LanceTrainer(lance) => format!("{:?}", lance),
            Trainer::MusTrainer => "Partida completa".to_owned(),
        }
    );
    println!("Tantos iniciales: {}:{}", tantos[0], tantos[1]);
    let trainer_config = TrainerConfig {
        iterations: args.iter,
        method,
    };
    let game_config = GameConfig {
        abstract_game: args.abstract_game,
        lance: args.lance,
    };

    let banco = BancoEstrategias::new();
    match trainer {
        Trainer::LanceTrainer(lance) => {
            let mut lance_game = LanceGame::new(lance, tantos, game_config.abstract_game);
            let mut cfr = banco.estrategia_lance_mut(lance).borrow_mut();
            trainer.train(&mut cfr, &mut lance_game, &trainer_config);
            drop(cfr);

            println!("Exportando estrategias...");
            let curr_time = Utc::now();
            output_path.push(format!("{}", curr_time.format("%Y-%m-%d %H:%M")));
            banco
                .export_estrategia(&output_path, lance, &trainer_config, &game_config)
                .expect("Error exportando estrategias.");
        }
        Trainer::MusTrainer => {
            banco
                .export(&output_path, &trainer_config, &game_config)
                .expect("Error exportando estrategias.");
        }
    }
}
