use std::fs;

use indicatif::{ProgressBar, ProgressStyle};
use musolver::{
    mus::{Accion, Baraja, Carta, Lance},
    solver::{BancoEstrategias, LanceGame},
    ActionNode,
};

fn load_action_tree(action_tree_path: &String) -> ActionNode<usize, Accion> {
    let contents = fs::read_to_string(action_tree_path).expect("Error reading the file.");
    let n: ActionNode<usize, Accion> = serde_json::from_str(&contents).unwrap();

    n
}

enum Trainer {
    LanceTrainer(Lance),
    MusTrainer,
}

#[derive(Debug, Clone, ValueEnum)]
enum CfrMethod {
    Cfr,
    CfrPlus,
    ChanceSampling,
    ExternalSampling,
}

struct TrainerConfig {
    method: CfrMethod,
    iterations: usize,
    action_tree_path: String,
    output_path: String,
    tantos: [u8; 2],
}

impl Trainer {
    fn crear_baraja() -> Baraja {
        let mut b = Baraja::new();
        for _ in 0..8 {
            b.insertar(Carta::As);
            b.insertar(Carta::Rey);
        }
        for _ in 0..4 {
            b.insertar(Carta::Caballo);
            b.insertar(Carta::Sota);
            b.insertar(Carta::Siete);
            b.insertar(Carta::Seis);
            b.insertar(Carta::Cinco);
            b.insertar(Carta::Cuatro);
        }
        b.barajar();
        b
    }

    fn train(&self, config: &TrainerConfig) {
        use std::time::Instant;

        let now = Instant::now();
        let pb = ProgressBar::new(config.iterations as u64);
        pb.set_style(
            ProgressStyle::with_template("{wide_bar:40.cyan/blue} {human_pos}/{human_len} {msg} ")
                .unwrap()
                .progress_chars("##-"),
        );
        let action_tree = load_action_tree(&config.action_tree_path);
        let banco = BancoEstrategias::new();
        let mut util = [0., 0.];
        let mut b = Self::crear_baraja();
        for i in 0..config.iterations {
            match self {
                Trainer::LanceTrainer(lance) => {
                    let p = LanceGame::new_random(&mut b, *lance, config.tantos);
                    let mut cfr = banco
                        .estrategia_lance_mut(*lance, p.tipo_estrategia())
                        .borrow_mut();
                    match config.method {
                        CfrMethod::Cfr => todo!(),
                        CfrMethod::CfrPlus => todo!(),
                        CfrMethod::ChanceSampling => {
                            util[0] += cfr.chance_cfr(&p, &action_tree, 0, 1., 1.);
                            util[1] += cfr.chance_cfr(&p, &action_tree, 1, 1., 1.);
                        }
                        CfrMethod::ExternalSampling => {
                            util[0] += cfr.external_cfr(&p, &action_tree, 0);
                            util[0] += cfr.external_cfr(&p, &action_tree, 1);
                        }
                    }
                }
                _ => {
                    //p = MusGame::new_random(&mut b, config.tantos);
                    //p.train(banco, &action_tree)
                }
            };

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
        println!("Exportando estrategias...");
        match self {
            Trainer::LanceTrainer(lance) => banco
                .export_estrategia_lance(*lance)
                .expect("Error exportando estrategias."),
            Trainer::MusTrainer => banco.export().expect("Error exportando estrategias."),
        }
    }
}

use clap::{Parser, ValueEnum};

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

    /// Ruta al fichero con el árbol de acciones a considerar en el cálculo del equilibrio.
    #[arg(short, long)]
    action_tree: Option<String>,

    /// Variante de CFR a utilizar.
    #[arg(short, long, value_enum)]
    method: Option<CfrMethod>,
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
    let action_tree_path = args
        .action_tree
        .unwrap_or_else(|| "config/action_tree.json".to_string());
    let method = args.method.unwrap_or(CfrMethod::ChanceSampling);

    println!("Musolver 0.1");
    println!(
        "Simulando: {}",
        match trainer {
            Trainer::LanceTrainer(lance) => format!("{:?}", lance),
            Trainer::MusTrainer => "Partida completa".to_owned(),
        }
    );
    println!("Tantos iniciales: {}:{}", tantos[0], tantos[1]);

    let config = TrainerConfig {
        iterations: args.iter,
        action_tree_path,
        method,
        output_path: "output/".to_string(),
        tantos,
    };

    trainer.train(&config);
}
