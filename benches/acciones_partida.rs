use std::path::Path;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use musolver::{
    ActionNode,
    mus::{Accion, Baraja, Lance, PartidaMus},
};

fn walk_tree(p: &PartidaMus, a: &ActionNode<usize, Accion>, history: &[Accion]) {
    match a {
        ActionNode::Terminal => {
            let mut new_partida = p.clone();
            history.iter().for_each(|a| {
                let _ = new_partida.actuar(*a);
            });
        }
        ActionNode::NonTerminal(_, next_actions) => {
            next_actions.iter().for_each(|(action, subtree)| {
                let mut new_history = history.to_vec();
                new_history.push(*action);
                walk_tree(p, subtree, &new_history);
            });
        }
    }
}

fn bench_acciones_partida(c: &mut Criterion) {
    c.bench_function("acciones_partida", |b| {
        b.iter_batched(
            || {
                let mut baraja = Baraja::baraja_mus();
                baraja.barajar();
                let manos = baraja.repartir_manos();
                let partida = PartidaMus::new_partida_lance(Lance::Grande, manos, [0, 0]).unwrap();
                let action_tree: ActionNode<usize, Accion> =
                    ActionNode::from_file(Path::new("config/action_tree.json"))
                        .expect("Error cargando árbol.");
                (partida, action_tree)
            },
            |(partida, action_tree)| {
                walk_tree(&partida, &action_tree, &Vec::new());
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_acciones_partida);
criterion_main!(benches);
