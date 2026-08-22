use std::fmt::Display;
use std::sync::Arc;

use arrayvec::ArrayVec;
use itertools::Itertools;

use crate::{
    Game, NodeType,
    mus::{
        Accion, Apuesta, Baraja, CuatroJugadores, DistribucionDobleCartaIter, EstadoLance,
        FaseEnvites, Juego, Lance, Mano, Pares, Turno,
    },
};

use super::{
    AbstractChica, AbstractGrande, AbstractJuego, AbstractPares, AbstractPunto, MusInfoSet,
    MusInfoSetTables,
};

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
        if self.idx_hands[0].0.is_multiple_of(2) {
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
fn hand_config_code(config: HandConfiguration) -> u64 {
    match config {
        HandConfiguration::CuatroManos => 0,
        HandConfiguration::TresManos1vs2 => 1,
        HandConfiguration::TresManos1vs2Intermedio => 2,
        HandConfiguration::TresManos2vs1 => 3,
        HandConfiguration::DosManos => 4,
        HandConfiguration::SinLance => 5,
    }
}

/// Anchura en bits de cada campo de la parte pública del conjunto de información.
const HAND_CONFIG_WIDTH: u32 = 3;
const HIDDEN_ACTION_WIDTH: u32 = 3;
/// Anchura de cada acción pública acumulada en el historial de apuestas.
const ACTION_WIDTH: u32 = 3;

#[derive(Debug, Clone)]
pub struct LanceGame {
    lance: Lance,
    tantos: [u8; 2],
    estado_lance: Option<EstadoLance<CuatroJugadores>>,
    pareja_mano: usize,
    abstract_game: bool,
    last_action: Option<Accion>,
    tables: Arc<MusInfoSetTables>,
    /// Parte privada del conjunto de información: la mano de cada jugador codificada para el lance.
    private_history: [u64; 4],
    /// Configuración de manos del lance (constante durante la partida).
    hand_config_code: u64,
    /// Acción oculta del primer miembro de la pareja, pendiente de respuesta del compañero.
    hidden_action: u64,
    /// Historial de apuestas: cada acción pública añade [`ACTION_WIDTH`] bits.
    history: u64,
}

impl LanceGame {
    pub fn new(lance: Lance, tantos: [u8; 2], abstract_game: bool) -> Self {
        Self {
            lance,
            tantos,
            abstract_game,
            estado_lance: None,
            last_action: None,
            pareja_mano: 0,
            tables: Arc::new(MusInfoSetTables::new(abstract_game)),
            private_history: [0; 4],
            hand_config_code: 0,
            hidden_action: 0,
            history: 0,
        }
    }

    pub fn new_with_configuration(&mut self, hand_configuration: HandConfiguration) {
        let jugadores = match hand_configuration {
            HandConfiguration::CuatroManos => vec![0, 1, 2, 3],
            HandConfiguration::TresManos1vs2 => vec![0, 1, 3],
            HandConfiguration::TresManos1vs2Intermedio => vec![0, 1, 2],
            HandConfiguration::TresManos2vs1 => vec![0, 2, 3],
            HandConfiguration::DosManos => vec![0, 1],
            HandConfiguration::SinLance => vec![0, 2],
        };
        let estado_lance = EstadoLance::<CuatroJugadores>::con_jugadores(
            &self.lance,
            &jugadores,
            [0, 0],
            0,
            FaseEnvites::<CuatroJugadores>::MAX_TANTOS,
        );
        self.pareja_mano = match estado_lance.turno().unwrap() {
            Turno::Pareja(idx) | Turno::Jugador(idx) => idx as usize,
        };
        self.hand_config_code = hand_config_code(hand_configuration);
        self.estado_lance = Some(estado_lance);
    }

    pub fn from_partida_mus(
        partida_mus: &FaseEnvites<CuatroJugadores>,
        abstract_game: bool,
    ) -> Option<Self> {
        let lance = partida_mus.lance_actual()?;
        let mut game = Self::new(lance, *partida_mus.tantos(), abstract_game);
        game.estado_lance = Some(EstadoLance::<CuatroJugadores>::new(
            &lance,
            partida_mus.manos(),
            FaseEnvites::<CuatroJugadores>::MAX_TANTOS,
        ));
        game.set_manos(partida_mus.manos());
        Some(game)
    }

    /// Fija en el conjunto de información la configuración de manos y la codificación de la mano de
    /// cada jugador para el lance en curso.
    fn set_manos(&mut self, manos: &[Mano; 4]) {
        let config = ManosNormalizadas::normalizar_mano(manos, &self.lance).hand_configuration();
        self.hand_config_code = hand_config_code(config);
        for (i, mano) in manos.iter().enumerate() {
            self.private_history[i] = self.tables.rank_hand(mano, &self.lance);
        }
    }

    /// Valor de la parte pública del conjunto de información: configuración de manos, acción oculta
    /// e historial de apuestas empaquetados en un `u64`.
    fn public_info_set(&self) -> u64 {
        self.hand_config_code
            | (self.hidden_action << HAND_CONFIG_WIDTH)
            | (self.history << (HAND_CONFIG_WIDTH + HIDDEN_ACTION_WIDTH))
    }

    // fn initialize_game(&mut self, manos: &[Mano; 4], turno_inicial: usize) {
    //     self.info_set_prefix = LanceGame::info_set_prefix(&p, self.abstract_game);
    //     self.estado_lance = Vec::with_capacity(6);
    //     self.estado_lance.push(p);
    //     self.pareja_mano = turno_inicial;
    // }

    pub fn actions(&self) -> ArrayVec<Accion, 6> {
        let partida = self.estado_lance.as_ref().unwrap();
        let turno = partida.turno().unwrap();
        let mut acciones = full_actions(partida.ultima_apuesta());
        if turno == Turno::Pareja(2) || turno == Turno::Pareja(3) {
            acciones.retain(|a| *a >= self.last_action.unwrap());
        }
        acciones
    }

    pub fn act_with_action(&mut self, a: Accion) {
        self.last_action = Some(a);
        let estado = self
            .estado_lance
            .as_ref()
            .expect("At least one EstadoLance must be available.");
        let turno = estado.turno().unwrap();
        // Índice de la acción en la lista completa (sin el recorte de pareja), que es el código con
        // el que se registra en el conjunto de información.
        let idx = full_actions(estado.ultima_apuesta())
            .iter()
            .position(|action| *action == a)
            .expect("the action must belong to the full action list") as u64;
        match turno {
            // El primer miembro de la pareja actúa a ciegas: su acción solo la conoce el compañero.
            Turno::Pareja(0) | Turno::Pareja(1) => {
                self.hidden_action = idx + 1;
            }
            // El compañero (o un jugador sin pareja) cierra la acción pública de la pareja.
            _ => {
                self.history = (self.history << ACTION_WIDTH) | (idx + 1);
                self.hidden_action = 0;
            }
        }
        let _ = self.estado_lance.as_mut().unwrap().actuar(a);
    }
}

/// Lista completa de acciones posibles ante la última apuesta, sin aplicar el recorte de la pareja.
/// Es la que fija el código de cada acción en el conjunto de información.
fn full_actions(ultimo_envite: Apuesta) -> ArrayVec<Accion, 6> {
    match ultimo_envite {
        Apuesta::Tantos(0) => [
            Accion::Paso,
            Accion::Envido(2),
            Accion::Envido(5),
            Accion::Envido(10),
            Accion::Ordago,
        ]
        .into_iter()
        .collect(),
        Apuesta::Tantos(2) => [
            Accion::Paso,
            Accion::Quiero,
            Accion::Envido(2),
            Accion::Envido(5),
            Accion::Envido(10),
            Accion::Ordago,
        ]
        .into_iter()
        .collect(),
        Apuesta::Tantos(4..=5) => [
            Accion::Paso,
            Accion::Quiero,
            Accion::Envido(10),
            Accion::Ordago,
        ]
        .into_iter()
        .collect(),
        Apuesta::Ordago => [Accion::Paso, Accion::Quiero].into_iter().collect(),
        _ => [Accion::Paso, Accion::Quiero, Accion::Ordago]
            .into_iter()
            .collect(),
    }
}

impl Game for LanceGame {
    type InfoSet = MusInfoSet;
    const N_PLAYERS: usize = 4;

    fn chance_sample(&self) -> Self {
        let mut new_game = self.clone();
        loop {
            // Baraja nueva en cada intento: `repartir_manos` consume las cartas, y en pares y juego
            // pueden hacer falta varios repartos hasta dar con uno en el que el lance se juegue.
            let mut baraja = Baraja::baraja_mus();
            let manos = baraja.repartir_manos();
            let intento_partida = EstadoLance::<CuatroJugadores>::new(
                &self.lance,
                &manos,
                FaseEnvites::<CuatroJugadores>::MAX_TANTOS,
            );
            // `turno_inicial` solo es válido cuando el lance se juega (en pares y juego hace falta
            // que alguna pareja ligue jugada), así que se calcula tras comprobar que hay turno.
            if intento_partida.turno().is_some() {
                new_game.estado_lance = Some(intento_partida);
                new_game.set_manos(&manos);
                new_game.pareja_mano = self.lance.turno_inicial(&manos);
                break;
            }
        }
        new_game
    }

    fn chance_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        DistribucionDobleCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS).flat_map(
            move |(_mano1, _mano2, prob)| {
                DistribucionDobleCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS).map(
                    move |(_mano3, _mano4, prob2)| {
                        // let manos = [
                        //     Mano::new(mano1.to_owned()),
                        //     Mano::new(mano2.to_owned()),
                        //     Mano::new(mano3),
                        //     Mano::new(mano4),
                        // ];
                        //let intento_partida = EstadoLance::<CuatroJugadores>::new(
                        //    &self.lance,
                        //    &manos,
                        //    PartidaMus::<CuatroJugadores>::MAX_TANTOS,
                        //);
                        //let turno_inicial = self.lance.turno_inicial(&manos);
                        (
                            Self::new(self.lance, self.tantos, self.abstract_game),
                            prob * prob2,
                        )
                        //if intento_partida.turno().is_some() {
                        //    let mut partida =
                        //        Self::new(self.lance, self.tantos, self.abstract_game);
                        //    partida.estado_lance = Some(intento_partida);
                        //    partida.info_set_prefix = LanceGame::info_set_prefix(
                        //        &self.lance,
                        //        &manos,
                        //        &self.tantos,
                        //        self.abstract_game,
                        //    );
                        //    partida.pareja_mano = turno_inicial;
                        //    (partida, prob * prob2)
                        //}
                    },
                )
            },
        )
    }

    // let mut iter = DistribucionDobleCartaIter::new(&Baraja::FREC_BARAJA_MUS, 4);
    // let mut frecuencia_baraja_2 = Baraja::FREC_BARAJA_MUS;
    // while let Some((mano1_pareja1, mano2_pareja1, probabilidad_pareja1)) = iter.next() {
    //     let manos_pareja1 = [Mano::new(mano1_pareja1), Mano::new(mano2_pareja1)];
    //     let frequencies2 = iter.current_frequencies();
    //     frecuencia_baraja_2
    //         .iter_mut()
    //         .zip(frequencies2.iter())
    //         .for_each(|(carta, f2)| {
    //             carta.1 = *f2 as u8;
    //         });
    //     let iter2 = DistribucionDobleCartaIter::new(&frecuencia_baraja_2, 4);
    //     for (mano1_pareja2, mano2_pareja2, probabilidad_pareja2) in iter2 {
    //         let manos = [
    //             manos_pareja1[0].clone(),
    //             Mano::new(mano1_pareja2),
    //             manos_pareja1[1].clone(),
    //             Mano::new(mano2_pareja2),
    //         ];
    //         let turno_inicial = self.lance.turno_inicial(&manos);
    //         let intento_partida = EstadoLance::<CuatroJugadores>::new(
    //             &self.lance,
    //             &manos,
    //             PartidaMus::<CuatroJugadores>::MAX_TANTOS,
    //         );
    //         if intento_partida.turno().is_some() {
    //             self.estado_lance = Some(intento_partida);
    //             self.info_set_prefix = LanceGame::info_set_prefix(
    //                 &self.lance,
    //                 &manos,
    //                 &self.tantos,
    //                 self.abstract_game,
    //             );
    //             self.pareja_mano = turno_inicial;
    //             f(self, probabilidad_pareja1 * probabilidad_pareja2);
    //         }
    //     }
    //}

    fn utility(&self, player: usize) -> f64 {
        let mut estado_lance = self.estado_lance.clone().unwrap();
        let ganador = estado_lance.resolver_lance();
        let tantos_ganador = match estado_lance.tantos_apostados() {
            Apuesta::Tantos(t) => t,
            Apuesta::Ordago => FaseEnvites::<CuatroJugadores>::MAX_TANTOS,
        } + estado_lance.tantos_mano()[ganador as usize];
        let mut tantos = self.tantos;
        if self.pareja_mano == 1 {
            tantos.swap(0, 1);
        }
        tantos[ganador as usize] += tantos_ganador;
        if tantos[ganador as usize] >= FaseEnvites::<CuatroJugadores>::MAX_TANTOS {
            tantos[ganador as usize] = FaseEnvites::<CuatroJugadores>::MAX_TANTOS;
            tantos[1 - ganador as usize] = 0;
        }
        let payoff = [
            tantos[0] as i8 - tantos[1] as i8,
            tantos[1] as i8 - tantos[0] as i8,
        ];
        payoff[player % 2] as f64
    }

    fn info_set(&self, player: usize) -> Self::InfoSet {
        (self.public_info_set(), self.private_history[player])
    }

    fn current_node(&self) -> NodeType {
        match &self.estado_lance {
            None => NodeType::Chance,
            Some(estado_lance) => match estado_lance.turno() {
                None => NodeType::Terminal,
                Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                    NodeType::Player(player_id as usize, self.actions().len())
                }
            },
        }
    }

    fn act(&self, action_id: usize) -> Self {
        let a = self.actions()[action_id];
        let mut new_game = self.clone();
        new_game.act_with_action(a);
        new_game
    }

    fn node_key(&self) -> u64 {
        self.public_info_set()
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

    /// Reparte un lance con [`LanceGame::chance_sample`] y juega una línea determinista hasta el
    /// nodo terminal, comprobando que la representación compacta no provoca pánicos.
    #[test]
    fn lance_game_info_set_no_panic() {
        for lance in [
            Lance::Grande,
            Lance::Chica,
            Lance::Pares,
            Lance::Juego,
            Lance::Punto,
        ] {
            for abstract_game in [false, true] {
                let mut game = LanceGame::new(lance, [0, 0], abstract_game).chance_sample();
                let mut terminado = false;
                for _ in 0..50 {
                    match game.current_node() {
                        NodeType::Terminal => {
                            terminado = true;
                            break;
                        }
                        NodeType::Chance => game = game.chance_sample(),
                        NodeType::Player(player, n_actions) => {
                            let _ = game.info_set(player);
                            game = game.act(n_actions - 1);
                        }
                    }
                }
                assert!(terminado, "el lance {lance:?} no terminó");
            }
        }
    }

    /// En el juego abstracto dos manos con la misma jugada en el lance comparten la parte privada
    /// del conjunto de información; en el juego exacto no.
    #[test]
    fn lance_game_abstract_merges_hands() {
        let opp = [
            Mano::try_from("RRR1").unwrap(),
            Mano::try_from("RRR4").unwrap(),
            Mano::try_from("RRR7").unwrap(),
        ];
        let manos_a = [
            Mano::try_from("S655").unwrap(),
            opp[0].clone(),
            opp[1].clone(),
            opp[2].clone(),
        ];
        let manos_b = [
            Mano::try_from("S544").unwrap(),
            opp[0].clone(),
            opp[1].clone(),
            opp[2].clone(),
        ];

        let mut abstracto_a = LanceGame::new(Lance::Grande, [0, 0], true);
        abstracto_a.set_manos(&manos_a);
        let mut abstracto_b = LanceGame::new(Lance::Grande, [0, 0], true);
        abstracto_b.set_manos(&manos_b);
        assert_eq!(abstracto_a.info_set(0), abstracto_b.info_set(0));

        let mut exacto_a = LanceGame::new(Lance::Grande, [0, 0], false);
        exacto_a.set_manos(&manos_a);
        let mut exacto_b = LanceGame::new(Lance::Grande, [0, 0], false);
        exacto_b.set_manos(&manos_b);
        assert_ne!(exacto_a.info_set(0), exacto_b.info_set(0));
    }
}
