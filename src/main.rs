use indicatif::{ProgressBar, ProgressStyle};
use mus::{Accion, Lance};
use musolver::*;

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

fn external_cfr(lance: Lance, tantos: [u8; 2], iter: usize) {
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
    for i in 0..iter {
        let p = PartidaLance::new_random(lance, tantos);
        let c = banco.estrategia_lance_mut(lance, p.tipo_estrategia());

        util[0] += c.external_cfr(&p, &action_tree, 0);
        util[1] += c.external_cfr(&p, &action_tree, 1);
        pb.inc(1);
        pb.set_message(format!(
            "Utility: {:.5} {:.5}",
            util[0] / (i as f32),
            util[1] / (i as f32),
        ));
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
    banco.export().expect("Error exportando estrategias.");
}

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    iter: usize,

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

fn main() {
    let args = Args::parse();

    let tantos = args.tantos.unwrap_or_default();

    external_cfr(Lance::Grande, tantos, args.iter);
}
