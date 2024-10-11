use std::path::Path;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use musolver::{
    mus::{Accion, Lance},
    solver::LanceGame,
    ActionNode, Cfr, CfrMethod,
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
                cfr.train(
                    &mut game,
                    &action_tree,
                    CfrMethod::ChanceSampling,
                    100,
                    |_, _| {},
                );
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
                cfr.train(
                    &mut game,
                    &action_tree,
                    CfrMethod::ExternalSampling,
                    100,
                    |_, _| {},
                );
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_chance_sampling, bench_external_sampling);
criterion_main!(benches);
