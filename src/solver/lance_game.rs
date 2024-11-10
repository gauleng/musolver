use std::fmt::Display;

use itertools::Itertools;

use crate::{
    mus::{
        Accion, Apuesta, Baraja, DistribucionDobleCartaIter, Juego, Lance, Mano, Pares, PartidaMus,
        Turno,
    },
    Game,
};

use super::{AbstractChica, AbstractGrande, AbstractJuego, AbstractPares, AbstractPunto};

/// Representación de las distintas configuraciones de las manos en un lance de mus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandConfiguration {
    /// Cuatro manos en juego: todos los jugadores participane en el lance. Es la única
    /// configuración posible en grande, chica y punto.
    CuatroManos,
    /// Tres manos en juego y el primero en hablar ese el jugador que no tiene pareja.
    TresManos1vs2,
    /// Tres manos en juego y el primero en hablar es el jugador que no tiene pareja, pero está
    /// situado entre los dos jugadores de la pareja rival.
    TresManos1vs2Intermedio,
    /// Tres manso en juego y habla primero la pareja.
    TresManos2vs1,
    /// Dos manos en juego.
    DosManos,
    /// El lance no se juega. Se corresponde con los casos en los que solo una pareja tiene
    /// jugadas.
    SinLance,
}

impl Display for HandConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandConfiguration::CuatroManos => write!(f, "2-2"),
            HandConfiguration::TresManos1vs2 => write!(f, "1-2"),
            HandConfiguration::TresManos1vs2Intermedio => write!(f, "1-1-1"),
            HandConfiguration::TresManos2vs1 => write!(f, "2-1"),
            HandConfiguration::DosManos => write!(f, "1-1"),
            HandConfiguration::SinLance => write!(f, "-"),
        }
    }
}

/// Representa las configuraciones de manos posibles en un lance de mus.
///
/// En los lances grande, chica y punto participan todos los jugadores, lo que se representa con la
/// variante CuatroManos.
///
/// En pares y juego el número de participantes depende de quién tenga jugada, por lo que pueden
/// darse las siguientes situaciones, en donde se numeran los jugadores participantes en un lance
/// con números del 0 al 3. Por ejemplo, 0-2-3 se refiere a que participan tres jugadores, el
/// jugador mano y su pareja, junto con el jugador postre.
///
/// * 0-1-2-3: CuatroManos.
/// * 0-1-2: TresManos1vs2Intermedio
/// * 1-2-3: TresManos1vs2Intermedio
/// * 0-2-3: TresManos2vs1
/// * 0-1-3: TresManos1vs2
/// * 0-1, 1-2, 2-3, 0-3: DosManos
///
/// En los casos de tres manos la convención es la siguiente:
/// * Si el jugador que está solo es el primero en hablar, es un caso 1vs2.
/// * Si el jugador que está solo es el último en hablar, es un caso 2vs1.
/// * Si está en una posición intermedia, es un caso 1vs2Intermedio. Se asume que al tener la
///   pareja un jugador que puede hablar de último, el mano siempre pasará dejando la voz al jugador
///   que está solo.
///
/// Esta estructura también dispone de un método para normalizar las manos de una mesa de mus. Se
/// puede usar como sigue:
///
///        use musolver::solver::{ManosNormalizadas, HandConfiguration};
///        use musolver::mus::{Mano, Lance};
///
///        let manos = [
///            Mano::try_from("RRRR").unwrap(),
///            Mano::try_from("RRR1").unwrap(),
///            Mano::try_from("RR11").unwrap(),
///            Mano::try_from("R111").unwrap(),
///        ];
///        let manos_normalizadas =
///            ManosNormalizadas::normalizar_mano(&manos, &Lance::Juego);
///
///        assert_eq!(manos_normalizadas.hand_configuration(), HandConfiguration::DosManos);
///
///        assert_eq!(manos_normalizadas.manos(0).0.to_string(), "RRRR");
///        assert!(manos_normalizadas.manos(0).1.is_none());
///
///        assert_eq!(manos_normalizadas.manos(1).0.to_string(), "RRR1");
///        assert!(manos_normalizadas.manos(1).1.is_none());
///
/// Dispone de métododos para convertir a String los pares de manos de cada jugador. Esta conversión
/// puede ser directa, es decir, las manos representadas por sus propias cartas, o puede ser
/// abstracta. En último caso, las manos se representan por sus jugadas. Por ejemplo, la mano RRR1
/// en el lance de juego se representa con "31F3".
///
/// Las abstracciones consideradas son las siguientes.
/// * Grande: Las dos mayores cartas de la mano. Por ejemplo, RRR1 pasa a ser RR.
/// * Chica: Las dos menores cartas de la mano. Por ejepmlo, RRR1 pasaa ser R1.
/// * Pares: Se utiliza las letras P, M, D para representar parejas, medias y duples
///   respectivamente. A continuación se indica el valor de las cartas que representan la jugada.
///   En el caso de duples, las dos parejas se denotan separadas por dos puntos. Por ejemplo, RRR1
///   pasa a ser M12, y RR11 pasa a ser D12:1.
/// * Juego: Se utiliza el valor de la mano, y en los casos en los que sea relevante, se indica el
///   número de figuras de la mano con una F. Por ejemplo, RRR1 pasa a ser 31F3, y R777 es 31F1.
/// * Punto: Se utiliza el valor de la mano.
pub struct ManosNormalizadas<'a> {
    hand_configuration: HandConfiguration,
    idx_hands: [(usize, Option<usize>); 2],
    hands: &'a [Mano; 4],
}

impl<'a> ManosNormalizadas<'a> {
    /// Permite normalizar las manos de una mesa de mus. Devuelve una configuración de manos de la
    /// partida junto con un array que contiene las manos agrupadas por parejas. Este array solo
    /// contiene las manos relevantes para el lance.
    pub fn normalizar_mano(m: &'a [Mano; 4], l: &Lance) -> Self {
        match l {
            Lance::Grande | Lance::Chica | Lance::Punto => {
                let idx_hands = [(0, Some(2)), (1, Some(3))];
                Self {
                    hand_configuration: HandConfiguration::CuatroManos,
                    idx_hands,
                    hands: m,
                }
            }
            Lance::Pares => {
                let jugadas: Vec<Option<Pares>> = m.iter().map(|m| m.pares()).collect();
                Self::normalizar_mano_jugadas(m, &jugadas)
            }
            Lance::Juego => {
                let jugadas: Vec<Option<Juego>> = m.iter().map(|m| m.juego()).collect();
                Self::normalizar_mano_jugadas(m, &jugadas)
            }
        }
    }

    fn normalizar_mano_jugadas<T>(manos: &'a [Mano; 4], jugadas: &[Option<T>]) -> Self {
        let (mut pareja_mano, mut pareja_postre): (Vec<_>, Vec<_>) = jugadas
            .iter()
            .enumerate()
            .filter_map(|(i, jugada)| jugada.as_ref().map(|_| i))
            .partition(|i| i % 2 == 0);
        if jugadas[1].is_some() && jugadas[2].is_some() && jugadas[3].is_none() {
            std::mem::swap(&mut pareja_mano, &mut pareja_postre);
        }
        let hand_configuration = match (pareja_mano.len(), pareja_postre.len()) {
            (2, 2) => HandConfiguration::CuatroManos,
            (1, 1) => HandConfiguration::DosManos,
            (2, 1) => HandConfiguration::TresManos2vs1,
            (1, 2) => {
                if jugadas[2].is_none() {
                    HandConfiguration::TresManos1vs2
                } else {
                    HandConfiguration::TresManos1vs2Intermedio
                }
            }
            _ => HandConfiguration::SinLance,
        };
        let idx_hands = [
            (pareja_mano[0], pareja_mano.get(1).cloned()),
            (pareja_postre[0], pareja_postre.get(1).cloned()),
        ];

        Self {
            hand_configuration,
            idx_hands,
            hands: manos,
        }
    }
    /// Devuelve un String con la representación de las dos manos separadas por una coma.
    pub fn par_manos_to_string(mano1: &Mano, mano2: Option<&Mano>) -> String {
        mano1.to_string() + "," + &mano2.map_or_else(|| "".to_owned(), |m| m.to_string())
    }

    /// Devuelve un String con la representación abstracta de una mano de mus.
    pub fn mano_to_abstract_string(m: &Mano, l: &Lance) -> String {
        match l {
            Lance::Grande => AbstractGrande::abstract_hand(m).to_string(),
            Lance::Chica => AbstractChica::abstract_hand(m).to_string(),
            Lance::Punto => AbstractPunto::abstract_hand(m).to_string(),
            Lance::Pares => {
                AbstractPares::abstract_hand(m).map_or("".to_string(), |p| p.to_string())
            }
            Lance::Juego => {
                AbstractJuego::abstract_hand(m).map_or("".to_string(), |j| j.to_string())
            }
        }
    }

    pub fn par_manos_to_abstract_string(mano1: &Mano, mano2: Option<&Mano>, l: &Lance) -> String {
        Self::mano_to_abstract_string(mano1, l)
            + ","
            + &mano2.map_or_else(|| "".to_string(), |m| Self::mano_to_abstract_string(m, l))
    }

    pub fn to_string_array(&self) -> [String; 2] {
        [
            Self::par_manos_to_string(self.manos(0).0, self.manos(0).1),
            Self::par_manos_to_string(self.manos(1).0, self.manos(1).1),
        ]
    }

    pub fn to_abstract_string_array(&self, l: &Lance) -> [String; 2] {
        [
            Self::par_manos_to_abstract_string(self.manos(0).0, self.manos(0).1, l),
            Self::par_manos_to_abstract_string(self.manos(1).0, self.manos(1).1, l),
        ]
    }

    /// Manos de la pareja mano o postre según el parámetro recibido.
    pub fn manos(&self, p: usize) -> (&Mano, Option<&Mano>) {
        let idx_player = self.idx_hands[p];
        (
            &self.hands[idx_player.0],
            idx_player.1.map(|idx| &self.hands[idx]),
        )
    }

    pub fn hand_configuration(&self) -> HandConfiguration {
        self.hand_configuration
    }

    pub fn pareja_mano(&self) -> usize {
        if self.idx_hands[0].0 % 2 == 0 {
            0
        } else {
            1
        }
    }
}

/// Estructura para generar las claves que representan los information sets durante el
/// entrenamiento.
pub struct InfoSet<'a> {
    pub tipo_estrategia: HandConfiguration,
    pub tantos: [u8; 2],
    pub manos: (&'a Mano, Option<&'a Mano>),
    pub history: Vec<Accion>,
    pub abstract_game: Option<Lance>,
}

impl<'a> InfoSet<'a> {
    pub fn str(
        hand_configuration: &HandConfiguration,
        tantos: &[u8; 2],
        mano1: &Mano,
        mano2: Option<&Mano>,
        history: &[Accion],
        abstract_game: Option<Lance>,
    ) -> String {
        let mut result = String::with_capacity(30);
        let manos_str = if let Some(lance) = abstract_game {
            ManosNormalizadas::par_manos_to_abstract_string(mano1, mano2, &lance)
        } else {
            ManosNormalizadas::par_manos_to_string(mano1, mano2)
        };
        let history_str = history.iter().map(|accion| accion.to_string()).join("");
        result.push_str(&tantos[0].to_string());
        result.push(':');
        result.push_str(&tantos[1].to_string());
        result.push(',');
        result.push_str(&hand_configuration.to_string());
        result.push(',');
        result.push_str(&manos_str);
        result.push(',');
        result.push_str(&history_str);
        result
    }
}

impl<'a> Display for InfoSet<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            Self::str(
                &self.tipo_estrategia,
                &self.tantos,
                self.manos.0,
                self.manos.1,
                &self.history,
                self.abstract_game
            )
        )
    }
}

/// Implementación del trait Game para un lance del mus en el que hay dos jugadores que cada uno
/// conoce las dos manos de una pareja.
///
/// Permite configurar el lance a jugar, los
/// tantos con los que empieza el marcador y si se va a considerar un lance abstracto (ver
/// HandConfiguration).
#[derive(Debug, Clone)]
pub struct LanceGame {
    lance: Lance,
    tantos: [u8; 2],
    partida: Vec<PartidaMus>,
    info_set_prefix: Option<[String; 4]>,
    pareja_mano: usize,
    abstract_game: bool,
    history: Vec<Accion>,
    history_str: Vec<Vec<String>>,
}

impl LanceGame {
    pub fn new(lance: Lance, tantos: [u8; 2], abstract_game: bool) -> Self {
        Self {
            lance,
            tantos,
            abstract_game,
            partida: vec![],
            info_set_prefix: None,
            history: Vec::with_capacity(12),
            history_str: vec![vec!["".into()]],
            pareja_mano: 0,
        }
    }

    pub fn from_partida_mus(partida_mus: &PartidaMus, abstract_game: bool) -> Option<Self> {
        Some(Self {
            lance: partida_mus.lance_actual()?,
            tantos: *partida_mus.tantos(),
            abstract_game,
            partida: vec![partida_mus.clone()],
            info_set_prefix: LanceGame::info_set_prefix(partida_mus, abstract_game),
            history: Vec::with_capacity(12),
            history_str: vec![vec!["".into()]],
            pareja_mano: 0,
        })
    }

    fn info_set_prefix(partida_mus: &PartidaMus, abstracto: bool) -> Option<[String; 4]> {
        let lance = partida_mus.lance_actual()?;
        let manos_normalizadas = ManosNormalizadas::normalizar_mano(partida_mus.manos(), &lance);
        let info_set_prefix: [String; 4] = core::array::from_fn(|i| {
            InfoSet::str(
                &manos_normalizadas.hand_configuration(),
                partida_mus.tantos(),
                &partida_mus.manos()[i],
                None,
                &[],
                if abstracto { Some(lance) } else { None },
            )
        });
        Some(info_set_prefix)
    }

    fn initialize_game(&mut self, p: PartidaMus, turno_inicial: usize) {
        self.info_set_prefix = LanceGame::info_set_prefix(&p, self.abstract_game);
        self.partida = Vec::with_capacity(6);
        self.partida.push(p);
        self.pareja_mano = turno_inicial;
    }
}

impl Game<usize, Accion> for LanceGame {
    fn new_random(&mut self) {
        let mut baraja = Baraja::baraja_mus();
        loop {
            baraja.barajar();
            let manos = baraja.repartir_manos();
            let mut tantos = self.tantos;
            let turno_inicial = self.lance.turno_inicial(&manos);
            if turno_inicial == 1 {
                tantos.swap(0, 1);
            }
            let intento_partida = PartidaMus::new_partida_lance(self.lance, manos, tantos);
            if let Some(p) = intento_partida {
                self.initialize_game(p, turno_inicial);
                break;
            }
        }
    }

    fn new_iter<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Self, f64),
    {
        let mut iter = DistribucionDobleCartaIter::new(&Baraja::FREC_BARAJA_MUS, 4);
        let mut frecuencia_baraja_2 = Baraja::FREC_BARAJA_MUS;
        while let Some((mano1_pareja1, mano2_pareja1, probabilidad_pareja1)) = iter.next() {
            let manos_pareja1 = [Mano::new(mano1_pareja1), Mano::new(mano2_pareja1)];
            let frequencies2 = iter.current_frequencies();
            frecuencia_baraja_2
                .iter_mut()
                .zip(frequencies2.iter())
                .for_each(|(carta, f2)| {
                    carta.1 = *f2 as u8;
                });
            let iter2 = DistribucionDobleCartaIter::new(&frecuencia_baraja_2, 4);
            for (mano1_pareja2, mano2_pareja2, probabilidad_pareja2) in iter2 {
                let manos = [
                    manos_pareja1[0].clone(),
                    Mano::new(mano1_pareja2),
                    manos_pareja1[1].clone(),
                    Mano::new(mano2_pareja2),
                ];
                let mut tantos = self.tantos;
                let turno_inicial = self.lance.turno_inicial(&manos);
                if turno_inicial == 1 {
                    tantos.swap(0, 1);
                }
                let intento_partida = PartidaMus::new_partida_lance(self.lance, manos, tantos);
                if let Some(p) = intento_partida {
                    self.initialize_game(p, turno_inicial);
                    f(self, probabilidad_pareja1 * probabilidad_pareja2);
                }
            }
        }
    }

    fn utility(&self, player: usize) -> f64 {
        let partida = self.partida.last().unwrap();
        let tantos = *partida.tantos();
        let payoff = [
            tantos[0] as i8 - tantos[1] as i8,
            tantos[1] as i8 - tantos[0] as i8,
        ];
        payoff[player % 2] as f64
    }

    fn info_set_str(&self, player: usize) -> String {
        let info_set_prefix = &self.info_set_prefix.as_ref().unwrap()[player];
        let mut output = String::with_capacity(15 + self.history.len() + 1);
        output.push_str(info_set_prefix);
        if let Some(history_str) = self.history_str.last() {
            for i in history_str {
                output.push_str(i);
            }
        }
        output
    }

    fn player_id(&self, idx: usize) -> usize {
        idx
    }

    fn num_players(&self) -> usize {
        4
    }

    fn actions(&self) -> Option<Vec<Accion>> {
        let partida = self.partida.last().unwrap();
        let turno = partida.turno()?;
        let ultimo_envite: Apuesta = partida.ultima_apuesta();
        let mut acciones = match ultimo_envite {
            Apuesta::Tantos(0) => vec![
                Accion::Paso,
                Accion::Envido(2),
                Accion::Envido(5),
                Accion::Envido(10),
                Accion::Ordago,
            ],
            Apuesta::Tantos(2) => vec![
                Accion::Paso,
                Accion::Quiero,
                Accion::Envido(2),
                Accion::Envido(5),
                Accion::Envido(10),
                Accion::Ordago,
            ],
            Apuesta::Tantos(4..=5) => vec![
                Accion::Paso,
                Accion::Quiero,
                Accion::Envido(10),
                Accion::Ordago,
            ],
            Apuesta::Ordago => vec![Accion::Paso, Accion::Quiero],
            _ => vec![Accion::Paso, Accion::Quiero, Accion::Ordago],
        };
        if turno == Turno::Pareja(2) || turno == Turno::Pareja(3) {
            acciones.retain(|a| a >= self.history.last().unwrap());
        }
        Some(acciones)
    }

    fn is_terminal(&self) -> bool {
        self.partida.last().unwrap().turno().is_none()
    }

    fn current_player(&self) -> Option<usize> {
        match self.partida.last().unwrap().turno()? {
            Turno::Jugador(player_id) | Turno::Pareja(player_id) => Some(player_id as usize),
        }
    }

    fn act(&mut self, a: Accion) {
        self.history.push(a);
        self.history_str
            .extend_from_within(self.history_str.len() - 1..);
        let last_history_str = self.history_str.last_mut().unwrap();
        let turno = self.partida.last().unwrap().turno().unwrap();
        match turno {
            Turno::Pareja(2) | Turno::Pareja(3) => {
                last_history_str.pop();
            }
            _ => {}
        };
        let action_str = match turno {
            Turno::Pareja(0) | Turno::Pareja(1) => a.to_string() + "*",
            _ => a.to_string(),
        };
        last_history_str.push(action_str);
        self.partida.extend_from_within(self.partida.len() - 1..);
        let _ = self.partida.last_mut().unwrap().actuar(a);
    }

    fn takeback(&mut self) {
        self.partida.pop();
        self.history_str.pop();
        self.history.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalizar() {
        let manos = [
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RR11").unwrap(),
            Mano::try_from("R111").unwrap(),
        ];
        let manos_normalizadas = ManosNormalizadas::normalizar_mano(&manos, &Lance::Juego);
        assert_eq!(
            manos_normalizadas.hand_configuration(),
            HandConfiguration::DosManos
        );
        assert_eq!(manos_normalizadas.manos(0).0.to_string(), "RRRR");
        assert!(manos_normalizadas.manos(0).1.is_none());
        assert_eq!(manos_normalizadas.manos(1).0.to_string(), "RRR1");
        assert!(manos_normalizadas.manos(1).1.is_none());

        let manos = [
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("R111").unwrap(),
        ];
        let manos_normalizadas = ManosNormalizadas::normalizar_mano(&manos, &Lance::Juego);
        assert_eq!(
            manos_normalizadas.hand_configuration(),
            HandConfiguration::TresManos1vs2Intermedio
        );
        assert_eq!(manos_normalizadas.manos(0).0.to_string(), "RRR1");
        assert!(manos_normalizadas.manos(0).1.is_none());
        assert_eq!(manos_normalizadas.manos(1).0.to_string(), "RRRR");
        assert!(manos_normalizadas.manos(1).1.is_some());

        let manos = [
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RRR1").unwrap(),
        ];
        let manos_normalizadas = ManosNormalizadas::normalizar_mano(&manos, &Lance::Juego);
        assert_eq!(
            manos_normalizadas.hand_configuration(),
            HandConfiguration::TresManos1vs2
        );
        assert_eq!(manos_normalizadas.manos(0).0.to_string(), "RRRR");
        assert!(manos_normalizadas.manos(0).1.is_none());
        assert_eq!(manos_normalizadas.manos(1).0.to_string(), "RRR1");
        assert!(manos_normalizadas.manos(1).1.is_some());

        let manos = [
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RRR1").unwrap(),
        ];
        let manos_normalizadas = ManosNormalizadas::normalizar_mano(&manos, &Lance::Juego);
        assert_eq!(
            manos_normalizadas.hand_configuration(),
            HandConfiguration::TresManos2vs1
        );
        assert_eq!(manos_normalizadas.manos(0).0.to_string(), "RRRR");
        assert!(manos_normalizadas.manos(0).1.is_some());
        assert_eq!(manos_normalizadas.manos(1).0.to_string(), "RRR1");
        assert!(manos_normalizadas.manos(1).1.is_none());
    }

    #[test]
    fn test_pareja_mano() {
        let manos = [
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RRR1").unwrap(),
        ];
        let manos_normalizadas = ManosNormalizadas::normalizar_mano(&manos, &Lance::Juego);
        assert_eq!(manos_normalizadas.pareja_mano(), 0);

        let manos = [
            Mano::try_from("RRRR").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("R111").unwrap(),
        ];
        let manos_normalizadas = ManosNormalizadas::normalizar_mano(&manos, &Lance::Juego);
        assert_eq!(manos_normalizadas.pareja_mano(), 1);

        let manos = [
            Mano::try_from("R111").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("R111").unwrap(),
        ];
        let manos_normalizadas = ManosNormalizadas::normalizar_mano(&manos, &Lance::Juego);
        assert_eq!(manos_normalizadas.pareja_mano(), 1);
    }
}
