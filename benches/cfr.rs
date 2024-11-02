use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use musolver::{mus::Lance, solver::LanceGame, Cfr, CfrMethod};

fn bench_chance_sampling_grande(c: &mut Criterion) {
    c.bench_function("chance_sampling grande", |b| {
        b.iter_batched(
            || {
                let game = LanceGame::new(Lance::Grande, [0, 0], false);
                let cfr = Cfr::new();
                (cfr, game)
            },
            |(mut cfr, mut game)| {
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
                let game = LanceGame::new(Lance::Juego, [0, 0], false);
                let cfr = Cfr::new();
                (cfr, game)
            },
            |(mut cfr, mut game)| {
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
                let game = LanceGame::new(Lance::Grande, [0, 0], false);
                let cfr = Cfr::new();
                (cfr, game)
            },
            |(mut cfr, mut game)| {
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
                let game = LanceGame::new(Lance::Juego, [0, 0], false);
                let cfr = Cfr::new();
                (cfr, game)
            },
            |(mut cfr, mut game)| {
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
