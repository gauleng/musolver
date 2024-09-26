use std::{
    fs::{self},
    path::{Path, PathBuf},
};

use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use musolver::{
    mus::{Accion, Lance},
    solver::{BancoEstrategias, LanceGame},
    ActionNode, Cfr, Game,
};

fn load_action_tree(action_tree_path: &Path) -> ActionNode<usize, Accion> {
    let contents = fs::read_to_string(action_tree_path).expect("Error reading the file.");
    let n: ActionNode<usize, Accion> = serde_json::from_str(&contents).unwrap();

    n
}

fn save_config(config: &TrainerConfig, path: &Path) {
    let contents = serde_json::to_string(config).expect("Error converting to JSON");
    fs::write(path, contents).expect("Error writing config");
}

enum Trainer {
    LanceTrainer(Lance),
    MusTrainer,
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
enum CfrMethod {
    Cfr,
    CfrPlus,
    ChanceSampling,
    ExternalSampling,
}

#[derive(Debug, Serialize, Deserialize)]
struct TrainerConfig {
    method: CfrMethod,
    iterations: usize,
    action_tree: ActionNode<usize, Accion>,
    tantos: [u8; 2],
}

impl Trainer {
    fn train<G>(&self, cfr: &mut Cfr<Accion>, game: &mut G, config: &TrainerConfig)
    where
        G: Game<usize, Accion>,
    {
        use std::time::Instant;

        let now = Instant::now();
        let pb = ProgressBar::new(config.iterations as u64);
        pb.set_style(
            ProgressStyle::with_template("{wide_bar:40.cyan/blue} {human_pos}/{human_len} {msg} ")
                .unwrap()
                .progress_chars("##-"),
        );
        let mut util = [0., 0.];
        for i in 0..config.iterations {
            game.new_random();
            match config.method {
                CfrMethod::Cfr => todo!(),
                CfrMethod::CfrPlus => todo!(),
                CfrMethod::ChanceSampling => {
                    util[0] += cfr.chance_cfr(game, &config.action_tree, 0, 1., 1.);
                    util[1] += cfr.chance_cfr(game, &config.action_tree, 1, 1., 1.);
                }
                CfrMethod::ExternalSampling => {
                    util[0] += cfr.external_cfr(game, &config.action_tree, 0);
                    util[1] += cfr.external_cfr(game, &config.action_tree, 1);
                }
            }

            pb.inc(1);
            if i % 1000 == 0 {
                pb.set_message(format!(
                    "Utility: {:.5} {:.5}",
                    util[0] / (i as f64),
                    util[1] / (i as f64),
                ));
            }
            // if i % 100000000 == 0 {
            //     banco
            //         .export_estrategia_lance(lance)
            //         .expect("Error exportando estrategias.");
            // }
        }
        let elapsed = now.elapsed();
        println!("Elapsed: {:.2?}", elapsed);
    }
}

use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

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
    let action_tree_path = PathBuf::from(
        args.action_tree
            .unwrap_or_else(|| "config/action_tree.json".to_string()),
    );
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
    let action_tree = load_action_tree(action_tree_path.as_path());
    let config = TrainerConfig {
        iterations: args.iter,
        action_tree,
        method,
        tantos,
    };

    let banco = BancoEstrategias::new();
    match trainer {
        Trainer::LanceTrainer(lance) => {
            let mut p = LanceGame::new(lance, config.tantos, args.abstract_game);
            let mut cfr = banco.estrategia_lance_mut(lance).borrow_mut();
            trainer.train(&mut cfr, &mut p, &config);
            drop(cfr);

            println!("Exportando estrategias...");
            let curr_time = Utc::now();
            output_path.push(format!("{}", curr_time.format("%Y-%m-%d %H:%M")));
            banco
                .export_estrategia(&output_path, lance)
                .expect("Error exportando estrategias.");
            let mut config_path = output_path.clone();
            config_path.push("config");
            config_path.set_extension("json");
            save_config(&config, config_path.as_path());
        }
        Trainer::MusTrainer => {
            banco
                .export(&output_path)
                .expect("Error exportando estrategias.");
        }
    }
}
