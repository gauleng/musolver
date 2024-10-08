use std::fmt::Display;

use itertools::Itertools;

use crate::{
    mus::{Accion, Baraja, DistribucionDobleCartaIter, Juego, Lance, Mano, Pares, PartidaMus},
    Game,
};

/// Representa las configuraciones de manos posibles en un lance de mus. En los lances grande,
/// chica y punto participan todos los jugadores, lo que se representa con la variante CuatroManos.
///
/// En pares y juego el número de participantes depende de quién tenga jugada, por lo que pueden
/// darse las siguientes situaciones, en donde se numeran los jugadores participantes en un lance
/// con números del 0 al 3. Por ejemplo, 0-2-3 se refiere a que participan tres jugadores, el
/// jugador mano y su pareja, junto con el jugador postre.
///
/// 0-1-2-3: CuatroManos.
/// 0-1-2: TresManos1vs2Intermedio
/// 1-2-3: TresManos1vs2Intermedio
/// 0-2-3: TresManos2vs1
/// 0-1-3: TresManos1vs2
/// 0-1, 1-2, 2-3, 0-3: DosManos
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
///        let manos = [
///            Mano::try_from("RRRR").unwrap(),
///            Mano::try_from("RRR1").unwrap(),
///            Mano::try_from("RR11").unwrap(),
///            Mano::try_from("R111").unwrap(),
///        ];
///        let (hand_config, manos_normalizadas) =
///            HandConfiguration::normalizar_mano(&manos, &Lance::Juego);
///
///        assert_eq!(hand_config, HandConfiguration::DosManos);
///
///        assert_eq!(manos_normalizadas.manos(0).0.to_string(), "RRRR");
///        assert!(manos_normalizadas.manos(0).1.is_none());
///
///        assert_eq!(manos_normalizadas.manos(1).0.to_string(), "RRR1");
///        assert!(manos_normalizadas.manos(1).1.is_none());
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandConfiguration {
    CuatroManos,
    TresManos1vs2,
    TresManos1vs2Intermedio,
    TresManos2vs1,
    DosManos,
}

impl<'a> HandConfiguration {
    /// Permite normalizar las manos de una mesa de mus. Devuelve una configuración de manos de la
    /// partida junto con un array que contiene las manos agrupadas por parejas. Este array solo
    /// contiene las manos relevantes para el lance.
    pub fn normalizar_mano(m: &'a [Mano], l: &Lance) -> (Self, ManosNormalizadas) {
        match l {
            Lance::Grande | Lance::Chica | Lance::Punto => {
                let mn = [
                    (m[0].clone(), Some(m[2].clone())),
                    (m[1].clone(), Some(m[3].clone())),
                ];
                (HandConfiguration::CuatroManos, ManosNormalizadas(mn))
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

    fn normalizar_mano_jugadas<T>(
        m: &'a [Mano],
        jugadas: &[Option<T>],
    ) -> (Self, ManosNormalizadas) {
        let mut parejas = [Vec::new(), Vec::new()];
        jugadas.iter().enumerate().for_each(|(i, p)| {
            if p.is_some() {
                parejas[i % 2].push(&m[i]);
            }
        });
        if jugadas[1].is_some() && jugadas[2].is_some() && jugadas[3].is_none() {
            parejas.swap(0, 1);
        }
        if parejas[0].len() == 2 && parejas[1].len() == 2 {
            let mn = [
                (m[0].clone(), Some(m[2].clone())),
                (m[1].clone(), Some(m[3].clone())),
            ];
            (HandConfiguration::CuatroManos, ManosNormalizadas(mn))
        } else if parejas[0].len() == 1 && parejas[1].len() == 1 {
            let mn = [(parejas[0][0].clone(), None), (parejas[1][0].clone(), None)];
            (HandConfiguration::DosManos, ManosNormalizadas(mn))
        } else if parejas[0].len() == 1 && parejas[1].len() == 2 {
            let tipo_estrategia = if jugadas[2].is_none() {
                HandConfiguration::TresManos1vs2
            } else {
                HandConfiguration::TresManos1vs2Intermedio
            };
            let mn = [
                (parejas[0][0].clone(), None),
                (parejas[1][0].clone(), Some(parejas[1][1].clone())),
            ];
            (tipo_estrategia, ManosNormalizadas(mn))
        } else {
            let mn = [
                (parejas[0][0].clone(), Some(parejas[0][1].clone())),
                (parejas[1][0].clone(), None),
            ];
            (HandConfiguration::TresManos2vs1, ManosNormalizadas(mn))
        }
    }
}

impl Display for HandConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandConfiguration::CuatroManos => write!(f, "2-2"),
            HandConfiguration::TresManos1vs2 => write!(f, "1-2"),
            HandConfiguration::TresManos1vs2Intermedio => write!(f, "1-1-1"),
            HandConfiguration::TresManos2vs1 => write!(f, "2-1"),
            HandConfiguration::DosManos => write!(f, "1-1"),
        }
    }
}

/// Estructura para almacenar las manos normalizadas de una mesa de mus.
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
pub struct ManosNormalizadas([(Mano, Option<Mano>); 2]);

impl ManosNormalizadas {
    /// Manos de la pareja mano o postre según el parámetro recibido.
    pub fn manos(&self, p: usize) -> &(Mano, Option<Mano>) {
        &self.0[p]
    }

    /// Devuelve un String con la representación de las dos manos separadas por una coma.
    pub fn par_manos_to_string(mano1: &Mano, mano2: Option<&Mano>) -> String {
        mano1.to_string() + "," + &mano2.map_or_else(|| "".to_owned(), |m| m.to_string())
    }

    /// Devuelve un String con la representación abstracta de una mano de mus.
    pub fn mano_to_abstract_string(m: &Mano, l: &Lance) -> String {
        match l {
            Lance::Grande | Lance::Chica => m.to_string(),
            Lance::Punto => m.valor_puntos().to_string(),
            Lance::Pares => m.pares().map_or_else(|| "".to_string(), |v| v.to_string()),
            Lance::Juego => m.juego().map_or_else(
                || "".to_string(),
                |v| match v {
                    Juego::Resto(_) | Juego::Treintaydos => v.to_string(),
                    Juego::Treintayuna => {
                        "31F".to_owned()
                            + &m.cartas()
                                .iter()
                                .filter(|c| c.valor() >= 10)
                                .count()
                                .to_string()
                    }
                },
            ),
        }
    }

    pub fn par_manos_to_abstract_string(mano1: &Mano, mano2: Option<&Mano>, l: &Lance) -> String {
        Self::mano_to_abstract_string(mano1, l)
            + ","
            + &mano2.map_or_else(|| "".to_string(), |m| Self::mano_to_abstract_string(m, l))
    }

    pub fn to_string_array(&self) -> [String; 2] {
        [
            Self::par_manos_to_string(&self.manos(0).0, self.manos(0).1.as_ref()),
            Self::par_manos_to_string(&self.manos(1).0, self.manos(1).1.as_ref()),
        ]
    }

    pub fn to_abstract_string_array(&self, l: &Lance) -> [String; 2] {
        [
            Self::par_manos_to_abstract_string(&self.manos(0).0, self.manos(0).1.as_ref(), l),
            Self::par_manos_to_abstract_string(&self.manos(1).0, self.manos(1).1.as_ref(), l),
        ]
    }
}

/// Estructura para generar las claves que representan los information sets durante el
/// entrenamiento.
pub struct InfoSet {
    pub tipo_estrategia: HandConfiguration,
    pub tantos: [u8; 2],
    pub manos: (Mano, Option<Mano>),
    pub history: Vec<Accion>,
    pub abstract_game: Option<Lance>,
}

impl InfoSet {
    pub fn str(
        hand_configuration: &HandConfiguration,
        tantos: &[u8; 2],
        mano1: &Mano,
        mano2: Option<&Mano>,
        history: &[Accion],
        abstract_game: Option<Lance>,
    ) -> String {
        format!(
            "{}:{},{},{},{}",
            tantos[0],
            tantos[1],
            hand_configuration,
            if let Some(lance) = abstract_game {
                ManosNormalizadas::par_manos_to_abstract_string(mano1, mano2, &lance)
            } else {
                ManosNormalizadas::par_manos_to_string(mano1, mano2)
            },
            history.iter().map(|accion| accion.to_string()).join(",")
        )
    }
}

impl Display for InfoSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{},{},{},{}",
            self.tantos[0],
            self.tantos[1],
            self.tipo_estrategia,
            if let Some(lance) = self.abstract_game {
                ManosNormalizadas::par_manos_to_abstract_string(
                    &self.manos.0,
                    self.manos.1.as_ref(),
                    &lance,
                )
            } else {
                ManosNormalizadas::par_manos_to_string(&self.manos.0, self.manos.1.as_ref())
            },
            self.history
                .iter()
                .map(|accion| accion.to_string())
                .join("")
        )
    }
}

/// Implementación del trait Game para un lance del mus. Permite configurar el lance a jugar, los
/// tantos con los que empieza el marcador y si se va a considerar un lance abstracto (ver
/// HandConfiguration).
#[derive(Debug)]
pub struct LanceGame {
    lance: Lance,
    tantos: [u8; 2],
    partida: Option<PartidaMus>,
    baraja: Baraja,
    info_set_prefix: Option<[String; 2]>,
    abstract_game: bool,
}

impl LanceGame {
    pub fn new(lance: Lance, tantos: [u8; 2], abstract_game: bool) -> Self {
        let baraja = Baraja::baraja_mus();
        Self {
            lance,
            tantos,
            baraja,
            abstract_game,
            partida: None,
            info_set_prefix: None,
        }
    }

    pub fn from_partida_mus(partida_mus: &PartidaMus, abstract_game: bool) -> Option<Self> {
        Some(Self {
            lance: partida_mus.lance_actual()?,
            tantos: *partida_mus.tantos(),
            baraja: Baraja::baraja_mus(),
            abstract_game,
            partida: Some(partida_mus.clone()),
            info_set_prefix: LanceGame::info_set_prefix(partida_mus, abstract_game),
        })
    }

    fn info_set_prefix(partida_mus: &PartidaMus, abstracto: bool) -> Option<[String; 2]> {
        let lance = partida_mus.lance_actual()?;
        let (tipo_estrategia, manos_normalizadas) =
            HandConfiguration::normalizar_mano(partida_mus.manos(), &lance);
        let info_set_prefix: [String; 2] = core::array::from_fn(|i| {
            InfoSet {
                tantos: *partida_mus.tantos(),
                tipo_estrategia,
                manos: manos_normalizadas.manos(i).to_owned(),
                history: vec![],
                abstract_game: if abstracto { Some(lance) } else { None },
            }
            .to_string()
        });
        Some(info_set_prefix)
    }
}

impl Game<usize, Accion> for LanceGame {
    fn new_random(&mut self) {
        loop {
            self.baraja.barajar();
            let manos = self.baraja.repartir_manos();
            let intento_partida = PartidaMus::new_partida_lance(self.lance, manos, self.tantos);
            if let Some(p) = intento_partida {
                self.info_set_prefix = LanceGame::info_set_prefix(&p, self.abstract_game);
                self.partida = Some(p);
                break;
            }
        }
    }

    fn new_iter<F>(&mut self, mut f: F)
    where
        F: FnMut(&Self, f64),
    {
        let mut iter = DistribucionDobleCartaIter::new(&Baraja::FREC_BARAJA_MUS, 4);
        let mut frecuencia_baraja_2 = Baraja::FREC_BARAJA_MUS;
        while let Some(mano_pareja1) = iter.next() {
            let frequencies2 = iter.current_frequencies();
            frecuencia_baraja_2
                .iter_mut()
                .zip(frequencies2.iter())
                .for_each(|(carta, f2)| {
                    carta.1 = *f2 as u8;
                });
            let iter2 = DistribucionDobleCartaIter::new(&frecuencia_baraja_2, 4);
            for mano_pareja2 in iter2 {
                let manos = [
                    Mano::new(mano_pareja1.0.clone()),
                    Mano::new(mano_pareja2.0.clone()),
                    Mano::new(mano_pareja1.1.clone()),
                    Mano::new(mano_pareja2.1.clone()),
                ];
                let intento_partida = PartidaMus::new_partida_lance(self.lance, manos, self.tantos);
                if let Some(p) = intento_partida {
                    self.info_set_prefix = LanceGame::info_set_prefix(&p, self.abstract_game);
                    self.partida = Some(p);
                    f(self, mano_pareja1.2 * mano_pareja2.2);
                }
            }
        }
    }

    fn utility(&self, player: usize, history: &[Accion]) -> f64 {
        let mut partida = self.partida.as_ref().unwrap().clone();
        let turno_inicial = partida.turno().unwrap();
        history.iter().for_each(|&a| {
            let _ = partida.actuar(a);
        });
        let mut tantos = *partida.tantos();
        if turno_inicial == 1 {
            tantos.swap(0, 1);
        }
        let payoff = [
            tantos[0] as i8 - tantos[1] as i8,
            tantos[1] as i8 - tantos[0] as i8,
        ];
        payoff[player] as f64
    }

    fn info_set_str(&self, player: usize, history: &[Accion]) -> String {
        let mut output = String::with_capacity(9 + history.len() + 1);
        output.push_str(&self.info_set_prefix.as_ref().unwrap()[player]);
        for i in history.iter() {
            output.push_str(&i.to_string());
        }
        output
    }

    fn player_id(&self, idx: usize) -> usize {
        idx
    }

    fn num_players(&self) -> usize {
        2
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
        let (hand_config, manos_normalizadas) =
            HandConfiguration::normalizar_mano(&manos, &Lance::Juego);
        assert_eq!(hand_config, HandConfiguration::DosManos);
        assert_eq!(manos_normalizadas.manos(0).0.to_string(), "RRRR");
        assert!(manos_normalizadas.manos(0).1.is_none());
        assert_eq!(manos_normalizadas.manos(1).0.to_string(), "RRR1");
        assert!(manos_normalizadas.manos(1).1.is_none());
    }
}
