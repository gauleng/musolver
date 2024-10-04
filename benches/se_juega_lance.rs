use criterion::{criterion_group, criterion_main, Criterion};
use musolver::mus::{Baraja, Lance};

fn bench_hay_lance_juego(c: &mut Criterion) {
    c.bench_function("se_juega_lance", |b| {
        b.iter_batched(
            || {
                let mut baraja = Baraja::baraja_mus();
                baraja.barajar();
                baraja.repartir_manos()
            },
            |manos| {
                Lance::Juego.hay_lance(&manos);
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_hay_lance_pares(c: &mut Criterion) {
    c.bench_function("se_juega_lance", |b| {
        b.iter_batched(
            || {
                let mut baraja = Baraja::baraja_mus();
                baraja.barajar();
                baraja.repartir_manos()
            },
            |manos| {
                Lance::Pares.hay_lance(&manos);
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_se_juega_lance_juego(c: &mut Criterion) {
    c.bench_function("se_juega_lance", |b| {
        b.iter_batched(
            || {
                let mut baraja = Baraja::baraja_mus();
                baraja.barajar();
                baraja.repartir_manos()
            },
            |manos| {
                Lance::Juego.se_juega(&manos);
            },
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_se_juega_lance_pares(c: &mut Criterion) {
    c.bench_function("se_juega_lance", |b| {
        b.iter_batched(
            || {
                let mut baraja = Baraja::baraja_mus();
                baraja.barajar();
                baraja.repartir_manos()
            },
            |manos| {
                Lance::Pares.se_juega(&manos);
            },
            criterion::BatchSize::SmallInput,
        )
    });
}
criterion_group!(
    benches,
    bench_se_juega_lance_juego,
    bench_se_juega_lance_pares,
    bench_hay_lance_juego,
    bench_hay_lance_pares
);
criterion_main!(benches);
