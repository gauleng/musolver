use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use musolver::{
    mus::{Baraja, Lance},
    solver::{HandConfiguration, InfoSet},
};

fn bench_info_set_prefix(c: &mut Criterion) {
    let mut group = c.benchmark_group("InfoSetStr");
    let mut baraja = Baraja::baraja_mus();
    for i in 0..10 {
        baraja.barajar();
        let manos = baraja.repartir_manos();
        let (tipo_estrategia, manos_normalizadas) =
            HandConfiguration::normalizar_mano(&manos, &Lance::Grande);
        group.bench_with_input(
            BenchmarkId::new("info_set_str object", i),
            &(tipo_estrategia, manos_normalizadas),
            |b, (tipo_estrategia, manos_normalizadas)| {
                b.iter(|| {
                    InfoSet {
                        tantos: [0, 0],
                        tipo_estrategia: *tipo_estrategia,
                        manos: manos_normalizadas.manos(0).to_owned(),
                        history: vec![],
                        abstract_game: None,
                    }
                    .to_string();
                });
            },
        );
        let (tipo_estrategia, manos_normalizadas) =
            HandConfiguration::normalizar_mano(&manos, &Lance::Grande);
        group.bench_with_input(
            BenchmarkId::new("info_set_str function", i),
            &(tipo_estrategia, manos_normalizadas),
            |b, (tipo_estrategia, manos_normalizadas)| {
                b.iter(|| {
                    InfoSet::str(
                        tipo_estrategia,
                        &[0, 0],
                        &manos_normalizadas.manos(0).0,
                        manos_normalizadas.manos(0).1.as_ref(),
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
