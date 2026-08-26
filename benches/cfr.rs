use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use musolver::{
    Cfr, CfrMethod,
    mus::Lance,
    solver::{LanceGame, MusGameTwoPlayers},
};

fn bench_chance_sampling_grande(c: &mut Criterion) {
    c.bench_function("chance_sampling grande", |b| {
        b.iter_batched(
            || {
                let game = LanceGame::new(Lance::Grande, [0, 0], false);
                let cfr = Cfr::new().method(CfrMethod::ChanceSampling);
                (cfr, game)
            },
            |(mut cfr, game)| {
                cfr.train(&game, 100);
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
                let cfr = Cfr::new().method(CfrMethod::ChanceSampling);
                (cfr, game)
            },
            |(mut cfr, game)| {
                cfr.train(&game, 100);
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
                let cfr = Cfr::new().method(CfrMethod::ExternalSampling);
                (cfr, game)
            },
            |(mut cfr, game)| {
                cfr.train(&game, 100);
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
                let cfr = Cfr::new().method(CfrMethod::ExternalSampling);
                (cfr, game)
            },
            |(mut cfr, game)| {
                cfr.train(&game, 100);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_external_sampling_mus_two_players(c: &mut Criterion) {
    c.bench_function("external_sampling mus_two_players", |b| {
        b.iter_batched(
            || {
                let game = MusGameTwoPlayers::new([38, 38], false, 0);
                let cfr = Cfr::new().method(CfrMethod::ExternalSampling);
                (cfr, game)
            },
            |(mut cfr, game)| {
                cfr.train(&game, 50000);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_chance_sampling_mus_two_players(c: &mut Criterion) {
    c.bench_function("chance_sampling mus_two_players", |b| {
        b.iter_batched(
            || {
                let game = MusGameTwoPlayers::new([38, 38], false, 0);
                let cfr = Cfr::new().method(CfrMethod::ChanceSampling);
                (cfr, game)
            },
            |(mut cfr, game)| {
                cfr.train(&game, 50000);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_fsicfr_mus_two_players(c: &mut Criterion) {
    c.bench_function("fsicfr mus_two_players", |b| {
        b.iter_batched(
            || {
                let game = MusGameTwoPlayers::new([38, 38], false, 0);
                let cfr = Cfr::new().method(CfrMethod::FsiCfr);
                (cfr, game)
            },
            |(mut cfr, game)| {
                cfr.train(&game, 50000);
            },
            BatchSize::SmallInput,
        )
    });
}

fn bench_exploitability(c: &mut Criterion) {
    c.bench_function("exploitability mus_two_players", |b| {
        b.iter_batched(
            || {
                let game = MusGameTwoPlayers::new([38, 38], false, 0);
                let mut cfr = Cfr::new().method(CfrMethod::FsiCfr);
                cfr.train(&game, 500);
                (cfr, game)
            },
            |(mut cfr, game)| {
                cfr.exploitability(&game);
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
    bench_external_sampling_mus_two_players,
    bench_chance_sampling_mus_two_players,
    bench_fsicfr_mus_two_players,
    bench_exploitability
);
criterion_main!(benches);
