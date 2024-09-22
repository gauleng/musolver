use indicatif::{ProgressBar, ProgressStyle};
use musolver::{
    mus::{Accion, Baraja, Carta, Lance},
    solver::{BancoEstrategias, LanceGame, MusGame},
    ActionNode,
};

fn init_action_tree() -> ActionNode<usize, Accion> {
    let mut n = ActionNode::<usize, Accion>::new(0);
    let p1paso = n.add_non_terminal_action(Accion::Paso, 1).unwrap();
    p1paso.add_terminal_action(Accion::Paso);
    let p2paso_envido = p1paso
        .add_non_terminal_action(Accion::Envido(2), 0)
        .unwrap();
    p2paso_envido.add_terminal_action(Accion::Paso);
    p2paso_envido.add_terminal_action(Accion::Quiero);
    let p1paso_envido_ordago = p2paso_envido
        .add_non_terminal_action(Accion::Ordago, 1)
        .unwrap();
    p1paso_envido_ordago.add_terminal_action(Accion::Paso);
    p1paso_envido_ordago.add_terminal_action(Accion::Quiero);
    let p2paso_ordago = p1paso.add_non_terminal_action(Accion::Ordago, 0).unwrap();
    p2paso_ordago.add_terminal_action(Accion::Paso);
    p2paso_ordago.add_terminal_action(Accion::Quiero);
    let p1envido = n.add_non_terminal_action(Accion::Envido(2), 1).unwrap();
    p1envido.add_terminal_action(Accion::Paso);
    p1envido.add_terminal_action(Accion::Quiero);
    let p2envido_ordago = p1envido.add_non_terminal_action(Accion::Ordago, 0).unwrap();
    p2envido_ordago.add_terminal_action(Accion::Paso);
    p2envido_ordago.add_terminal_action(Accion::Quiero);
    let p1ordago = n.add_non_terminal_action(Accion::Ordago, 1).unwrap();
    p1ordago.add_terminal_action(Accion::Paso);
    p1ordago.add_terminal_action(Accion::Quiero);

    n
}

fn crear_baraja() -> Baraja {
    let mut b = Baraja::new();
    for _ in 0..8 {
        b.insertar(Carta::As);
        b.insertar(Carta::Rey);
    }
    for _ in 0..4 {
        b.insertar(Carta::Caballo);
        b.insertar(Carta::Sota);
        // b.insertar(Carta::Siete);
        // b.insertar(Carta::Seis);
        // b.insertar(Carta::Cinco);
        b.insertar(Carta::Cuatro);
    }
    b.barajar();
    b
}

fn external_cfr(lance: Trainer, tantos: [u8; 2], iter: usize) {
    use std::time::Instant;

    let action_tree = init_action_tree();

    let now = Instant::now();
    let pb = ProgressBar::new(iter as u64);
    pb.set_style(
        ProgressStyle::with_template("{wide_bar:40.cyan/blue} {human_pos}/{human_len} {msg} ")
            .unwrap()
            .progress_chars("##-"),
    );
    let mut banco = BancoEstrategias::new();
    let mut util = [0., 0.];
    let mut b = crear_baraja();
    for i in 0..iter {
        let (b, u) = match lance {
            Trainer::LanceTrainer(lance) => {
                let mut p = LanceGame::new_random(&mut b, lance, tantos);
                p.train(banco, &action_tree)
            }
            Trainer::MusTrainer => {
                let mut p = MusGame::new_random(&mut b, tantos);
                p.train(banco, &action_tree)
            }
        };
        banco = b;

        util[0] += u[0];
        util[1] += u[1];
        pb.inc(1);
        if i % 1000 == 0 {
            pb.set_message(format!(
                "Utility: {:.5} {:.5}",
                util[0] / (i as f32),
                util[1] / (i as f32),
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
    match lance {
        Trainer::LanceTrainer(lance) => banco
            .export_estrategia_lance(lance)
            .expect("Error exportando estrategias."),
        Trainer::MusTrainer => banco.export().expect("Error exportando estrategias."),
    }
}

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    iter: usize,

    #[arg(short, long, value_enum)]
    lance: Option<Lance>,

    #[arg(short, long, value_parser = parse_tantos)]
    tantos: Option<[u8; 2]>,
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

enum Trainer {
    LanceTrainer(Lance),
    MusTrainer,
}

fn main() {
    let args = Args::parse();

    let tantos = args.tantos.unwrap_or_default();
    let trainer = args
        .lance
        .map_or_else(|| Trainer::MusTrainer, Trainer::LanceTrainer);

    println!("Musolver 0.1");
    println!(
        "Simulando: {}",
        match trainer {
            Trainer::LanceTrainer(lance) => format!("{:?}", lance),
            Trainer::MusTrainer => "Partida completa".to_owned(),
        }
    );
    println!("Tantos iniciales: {}:{}", tantos[0], tantos[1]);

    external_cfr(trainer, tantos, args.iter);
}
