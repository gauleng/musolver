use indicatif::{ProgressBar, ProgressStyle};
use mus::{Accion, Lance};
use musolver::*;

fn help() {
    println!("Use: musolver <num iterations>");
}

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

fn external_cfr(iter: usize) {
    use std::time::Instant;

    let mut c = Cfr::new();
    let action_tree = init_action_tree();

    let now = Instant::now();
    let pb = ProgressBar::new(iter as u64);
    pb.set_style(
        ProgressStyle::with_template("{wide_bar:40.cyan/blue} {human_pos}/{human_len} {msg} ")
            .unwrap()
            .progress_chars("##-"),
    );
    let mut util = [0., 0.];
    for _ in 0..iter {
        let p = PartidaLance::new_random(Lance::Punto, [35, 0]);
        c.set_partida_lance(p);
        util[0] += c.cfr(&action_tree, 0);
        util[1] += c.cfr(&action_tree, 1);
        pb.inc(1);
        pb.set_message(format!("Utility: {:.5} {:.5}", util[0], util[1],));
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
    let mut v: Vec<(String, Node)> = c
        .nodes()
        .iter()
        .map(|(s, n)| (s.clone(), n.clone()))
        .collect();
    v.sort_by(|x, y| x.0.cmp(&y.0));
    for (k, n) in v {
        println!(
            "{},{}",
            k,
            n.get_average_strategy()
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>()
                .join(",")
        );
    }
}

fn main() {
    // let mut n = ActionNode::<usize, Accion>::new(0);
    // let p1 = n.add_non_terminal_action(Accion::Paso, 1).unwrap();
    // p1.add_terminal_action(Accion::Paso);
    // let p2 = p1.add_non_terminal_action(Accion::Envido(2), 0).unwrap();
    // p2.add_terminal_action(Accion::Quiero);
    // let p3 = n.add_non_terminal_action(Accion::Envido(2), 1).unwrap();
    // p3.add_terminal_action(Accion::Paso);
    // p3.add_terminal_action(Accion::Quiero);

    let args: Vec<String> = std::env::args().collect();
    match args.len() {
        0 | 1 => help(),
        2 => {
            let num_iter: usize = match args[1].parse() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("Second argument is not an integer.");
                    help();
                    return;
                }
            };
            external_cfr(num_iter);
        }
        _ => {}
    }
}
