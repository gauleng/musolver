use std::path::Path;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use musolver::{
    mus::{Accion, Lance},
    solver::LanceGameDosManos,
    ActionNode, Cfr, CfrMethod,
};

fn bench_chance_sampling_grande(c: &mut Criterion) {
    c.bench_function("chance_sampling grande", |b| {
        b.iter_batched(
            || {
                let game = LanceGameDosManos::new(Lance::Grande, [0, 0], false);
                let action_tree: ActionNode<usize, Accion> =
                    ActionNode::from_file(Path::new("config/action_tree.json"))
                        .expect("Error cargando árbol.");
                let cfr = Cfr::new();
                (cfr, game, action_tree)
            },
            |(mut cfr, mut game, action_tree)| {
                cfr.train(&mut game, CfrMethod::ChanceSampling, 100, |_, _| {});
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_chance_sampling_juego(c: &mut Criterion) {
    c.bench_function("chance_sampling juego", |b| {
        b.iter_batched(
            || {
                let game = LanceGameDosManos::new(Lance::Juego, [0, 0], false);
                let action_tree: ActionNode<usize, Accion> =
                    ActionNode::from_file(Path::new("config/action_tree.json"))
                        .expect("Error cargando árbol.");
                let cfr = Cfr::new();
                (cfr, game, action_tree)
            },
            |(mut cfr, mut game, action_tree)| {
                cfr.train(&mut game, CfrMethod::ChanceSampling, 100, |_, _| {});
            },
            BatchSize::SmallInput,
        )
    });
}
fn bench_external_sampling_grande(c: &mut Criterion) {
    c.bench_function("external_sampling grande", |b| {
        b.iter_batched(
            || {
                let game = LanceGameDosManos::new(Lance::Grande, [0, 0], false);
                let action_tree: ActionNode<usize, Accion> =
                    ActionNode::from_file(Path::new("config/action_tree.json"))
                        .expect("Error cargando árbol.");
                let cfr = Cfr::new();
                (cfr, game, action_tree)
            },
            |(mut cfr, mut game, action_tree)| {
                cfr.train(&mut game, CfrMethod::ExternalSampling, 100, |_, _| {});
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_external_sampling_juego(c: &mut Criterion) {
    c.bench_function("external_sampling juego", |b| {
        b.iter_batched(
            || {
                let game = LanceGameDosManos::new(Lance::Juego, [0, 0], false);
                let action_tree: ActionNode<usize, Accion> =
                    ActionNode::from_file(Path::new("config/action_tree.json"))
                        .expect("Error cargando árbol.");
                let cfr = Cfr::new();
                (cfr, game, action_tree)
            },
            |(mut cfr, mut game, action_tree)| {
                cfr.train(&mut game, CfrMethod::ExternalSampling, 100, |_, _| {});
            },
            BatchSize::SmallInput,
        )
    });
}
criterion_group!(
    benches,
    bench_chance_sampling_grande,
    bench_chance_sampling_juego,
    bench_external_sampling_grande,
    bench_external_sampling_juego,
);
criterion_main!(benches);
