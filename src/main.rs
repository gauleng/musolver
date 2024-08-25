use indicatif::ProgressBar;
use mus::{Accion, Baraja, Carta, Mano};
use musolver::*;

fn crear_baraja() -> Baraja {
    let mut b = Baraja::new();
    for _ in 0..8 {
        b.insertar(mus::Carta::As);
        b.insertar(mus::Carta::Rey);
    }
    for _ in 0..4 {
        b.insertar(mus::Carta::Caballo);
        // b.insertar(mus::Carta::Sota);
        // b.insertar(mus::Carta::Siete);
        // b.insertar(mus::Carta::Seis);
        // b.insertar(mus::Carta::Cinco);
        b.insertar(mus::Carta::Cuatro);
    }
    b.barajar();
    b
}

fn repartir_manos(mut b: Baraja) -> Vec<Mano> {
    let mut manos = Vec::with_capacity(4);
    for _ in 0..4 {
        let mut m = Vec::<Carta>::with_capacity(4);
        for _ in 0..4 {
            m.push(b.repartir().unwrap());
        }
        manos.push(Mano::new(m));
    }
    manos
}

fn help() {
    println!("Use: musolver <num iterations>");
}

fn init_action_tree() -> ActionNode2<usize, Accion> {
    let mut n = ActionNode2::<usize, Accion>::new(0);
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
    let mut baraja = crear_baraja();
    let pb = ProgressBar::new(iter as u64);
    for _ in 0..iter {
        baraja.barajar();
        let b = baraja.clone();
        let m = repartir_manos(b);
        c.set_hands(m);
        c.cfr(&action_tree, 0);
        c.cfr(&action_tree, 1);
        pb.inc(1);
        //println!("{:?}", c.nodos);
    }
    let elapsed = now.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
    let mut v: Vec<(String, Node)> = c
        .nodes()
        .into_iter()
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
