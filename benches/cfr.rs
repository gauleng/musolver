use std::path::Path;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use musolver::{
    mus::{Accion, Lance},
    solver::LanceGame,
    ActionNode, Cfr, Game,
};

fn bench_chance_sampling(c: &mut Criterion) {
    c.bench_function("chance_sampling", |b| {
        b.iter_batched(
            || {
                let game = LanceGame::new(Lance::Grande, [0, 0], true);
                let action_tree: ActionNode<usize, Accion> =
                    ActionNode::from_file(Path::new("config/action_tree.json"))
                        .expect("Error cargando árbol.");
                let cfr = Cfr::new();
                (cfr, game, action_tree)
            },
            |(mut cfr, mut game, action_tree)| {
                game.new_random();
                cfr.chance_sampling(&game, &action_tree, 0, 1., 1.);
                cfr.chance_sampling(&game, &action_tree, 1, 1., 1.);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_external_sampling(c: &mut Criterion) {
    c.bench_function("external_sampling", |b| {
        b.iter_batched(
            || {
                let game = LanceGame::new(Lance::Grande, [0, 0], true);
                let action_tree: ActionNode<usize, Accion> =
                    ActionNode::from_file(Path::new("config/action_tree.json"))
                        .expect("Error cargando árbol.");
                let cfr = Cfr::new();
                (cfr, game, action_tree)
            },
            |(mut cfr, mut game, action_tree)| {
                game.new_random();
                cfr.external_sampling(&game, &action_tree, 0);
                cfr.external_sampling(&game, &action_tree, 1);
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_chance_sampling, bench_external_sampling);
criterion_main!(benches);
