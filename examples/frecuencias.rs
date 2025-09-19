use musolver::mus::{Baraja, Carta, DistribucionCartaIter, Jugada, Lance, Mano, RankingManos};

fn main() {
    let mut frecuencias_31f3 = Baraja::FREC_BARAJA_MUS;
    frecuencias_31f3[0].1 = 7;
    frecuencias_31f3[7].1 = 5;
    let probabilidades_juego = probabilidades_rivales(frecuencias_31f3, &Lance::Juego);
    println!("Contra 31 3 figuras");
    probabilidades_juego
        .iter()
        .for_each(|(juego, prob)| println!("{juego:?}: {prob}"));

    let mut frecuencias_31f2 = Baraja::FREC_BARAJA_MUS;
    frecuencias_31f2[1].1 = 3;
    frecuencias_31f2[4].1 = 3;
    frecuencias_31f2[7].1 = 6;
    let probabilidades_juego = probabilidades_rivales(frecuencias_31f2, &Lance::Juego);
    println!("Contra 31 2 figuras, 4 y 7");
    probabilidades_juego
        .iter()
        .for_each(|(juego, prob)| println!("{juego:?}: {prob}"));

    let mut frecuencias_31f2bis = Baraja::FREC_BARAJA_MUS;
    frecuencias_31f2bis[2].1 = 3;
    frecuencias_31f2bis[3].1 = 3;
    frecuencias_31f2bis[7].1 = 6;
    let probabilidades_juego = probabilidades_rivales(frecuencias_31f2bis, &Lance::Juego);
    println!("Contra 31 2 figuras, 5 y 6");
    probabilidades_juego
        .iter()
        .for_each(|(juego, prob)| println!("{juego:?}: {prob}"));

    let mut frecuencias_31f1 = Baraja::FREC_BARAJA_MUS;
    frecuencias_31f1[4].1 = 1;
    frecuencias_31f1[7].1 = 7;
    let probabilidades_juego = probabilidades_rivales(frecuencias_31f1, &Lance::Juego);
    println!("Contra 31 1 figura");
    probabilidades_juego
        .iter()
        .for_each(|(juego, prob)| println!("{juego:?}: {prob}"));
}

fn probabilidades_rivales(frecuencias: [(Carta, u8); 8], lance: &Lance) -> Vec<(Jugada, f64)> {
    let mut probabilidades: Vec<(Mano, f64)> = DistribucionCartaIter::new(&frecuencias, 4)
        .map(|(cartas, freq)| (Mano::new(cartas), freq))
        .filter(|(mano, _)| mano.jugada(lance).is_some())
        .collect();
    probabilidades.sort_by(|a, b| lance.compara_manos(&a.0, &b.0));
    let probabilidades_jugadas: Vec<(Jugada, f64)> = probabilidades
        .chunk_by(|a, b| a.0.jugada(lance) == b.0.jugada(lance))
        .map(|chunk| {
            (
                chunk[0].0.jugada(lance).unwrap(),
                chunk.iter().fold(0., |acc, v| acc + v.1),
            )
        })
        .collect();
    let probabilidad_total: f64 = probabilidades_jugadas
        .iter()
        .fold(0., |acc, &(_, probabilidad)| acc + probabilidad);
    let probabilidad_normalizada: Vec<(Jugada, f64)> = probabilidades_jugadas
        .into_iter()
        .map(|(jugada, probabilidad)| (jugada, probabilidad / probabilidad_total))
        .collect();
    probabilidad_normalizada
}
