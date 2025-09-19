use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use musolver::{
    mus::{Baraja, Lance},
    solver::{InfoSet, ManosNormalizadas},
};

fn bench_info_set_prefix(c: &mut Criterion) {
    let mut group = c.benchmark_group("InfoSetStr");
    let mut baraja = Baraja::baraja_mus();
    for i in 0..10 {
        baraja.barajar();
        let manos = baraja.repartir_manos();
        let manos_normalizadas = ManosNormalizadas::normalizar_mano(&manos, &Lance::Grande);
        group.bench_with_input(
            BenchmarkId::new("info_set_str object", i),
            &manos_normalizadas,
            |b, manos_normalizadas| {
                b.iter(|| {
                    InfoSet {
                        tantos: [0, 0],
                        tipo_estrategia: manos_normalizadas.hand_configuration(),
                        manos: manos_normalizadas.manos(0),
                        history: vec![],
                        abstract_game: None,
                    }
                    .to_string();
                });
            },
        );
        let manos_normalizadas = ManosNormalizadas::normalizar_mano(&manos, &Lance::Grande);
        group.bench_with_input(
            BenchmarkId::new("info_set_str function", i),
            &manos_normalizadas,
            |b, manos_normalizadas| {
                b.iter(|| {
                    InfoSet::str(
                        &manos_normalizadas.hand_configuration(),
                        &[0, 0],
                        manos_normalizadas.manos(0).0,
                        manos_normalizadas.manos(0).1,
                        &[],
                        None,
                    )
                    .to_string();
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_info_set_prefix);
criterion_main!(benches);
