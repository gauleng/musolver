use std::{fmt::Write, rc::Rc};

use arrayvec::ArrayString;
use itertools::{Either, Itertools};

use crate::{
    Game, NodeType,
    mus::{
        Accion, Apuesta, Baraja, Carta, CuatroJugadores, DistribucionCartaIter,
        DistribucionDobleCartaIter, DosJugadores, FasePartida, Lance, Mano, ModalidadMus,
        PartidaMus, Turno,
    },
    solver::ManosNormalizadas,
};

/// Número máximo de rondas de mus admitido por [`MusGame`], [`MusGameTwoHands`] y
/// [`MusGameTwoPlayers`]. El límite viene dado por el tamaño de los buffers del conjunto de
/// información: cada ronda añade un descarte por mano y unas pocas acciones al historial. Subirlo
/// obliga a agrandar `descarte_str` (5 bytes por mano y ronda) y `history_str`.
pub const MAX_RONDAS_MUS: u8 = 4;

#[derive(Debug, Clone)]
pub struct MusGame {
    tantos: [u8; 2],
    cards: Option<CardSource>,
    partida: Option<PartidaMus<CuatroJugadores>>,
    history_str: ArrayString<192>,
    info_set_prefix: [ArrayString<16>; 4],
    // Una mano por jugador, hasta 5 bytes por descarte y ronda.
    descarte_str: [ArrayString<{ 5 * MAX_RONDAS_MUS as usize }>; 4],
    last_action: Option<Accion>,
    manos_pares: ArrayString<4>,
    manos_juego: ArrayString<4>,
    mus_rounds: u8,
    max_mus_rounds: u8,
    abstract_game: bool,
    utility_table: Option<Rc<[[f64; 40]; 40]>>,
}

impl MusGame {
    /// `max_mus_rounds` acota cuántas rondas de mus pueden jugarse, hasta [`MAX_RONDAS_MUS`]. Con
    /// cero rondas la partida se juega a primeras dadas y la fase de mus no forma parte del
    /// árbol de juego.
    pub fn new(tantos: [u8; 2], abstract_game: bool, max_mus_rounds: u8) -> Self {
        assert!(
            max_mus_rounds <= MAX_RONDAS_MUS,
            "El máximo de rondas de mus es {MAX_RONDAS_MUS}, se han pedido {max_mus_rounds}."
        );
        Self {
            partida: None,
            cards: None,
            tantos,
            history_str: ArrayString::new(),
            info_set_prefix: [ArrayString::new(); 4],
            descarte_str: [ArrayString::new(); 4],
            last_action: None,
            manos_pares: ArrayString::new(),
            manos_juego: ArrayString::new(),
            mus_rounds: 0,
            max_mus_rounds,
            abstract_game,
            utility_table: None,
        }
    }

    pub fn with_hands(self, manos: [Mano; 4]) -> Self {
        let mut new_game = self.clone();
        new_game.set_hands(manos);
        new_game
    }

    fn set_hands(&mut self, manos: [Mano; 4]) {
        self.partida = Some(PartidaMus::<CuatroJugadores>::new(manos, self.tantos));
        self.actualizar_manos(Lance::Grande);
        // La 'M' marca el reparto, no la fase de mus: distingue la partida repartida del nodo
        // de azar inicial, que es el que `GameGraph` indexa con el historial vacío.
        self.history_str = ArrayString::<192>::from("M").unwrap();
        self.enforce_max_mus_rounds();
    }

    /// Finish mus phase when rounds reach max_mus_rounds.
    fn enforce_max_mus_rounds(&mut self) {
        if self.mus_rounds < self.max_mus_rounds {
            return;
        }
        let partida = self
            .partida
            .as_mut()
            .expect("La partida debe estar repartida.");
        if !matches!(partida.fase(), Some(FasePartida::Mus)) {
            return;
        }
        let _ = partida.actuar(Accion::NoMus);
        if let Some(FasePartida::Envites(lance)) = partida.fase() {
            self.actualizar_manos(lance);
        }
    }

    /// Refresca el prefijo del conjunto de información y los indicadores de pares y juego con las
    /// manos que hay ahora sobre la mesa. Se llama al entrar en la fase de envites porque los
    /// descartes pueden haber cambiado las manos repartidas.
    fn actualizar_manos(&mut self, lance: Lance) {
        let manos = self
            .partida
            .as_ref()
            .expect("La partida debe estar repartida.")
            .manos();
        self.info_set_prefix =
            MusGame::info_set_prefix(manos, &self.tantos, self.abstract_game.then_some(lance));
        (self.manos_pares, self.manos_juego) = jugadas_manos(manos);
    }

    fn set_card_source(&mut self, cartas: CardSource) {
        self.cards = Some(cartas);
    }

    fn info_set_prefix(
        manos: &[Mano; 4],
        tantos: &[u8; 2],
        abstracto: Option<Lance>,
    ) -> [ArrayString<16>; 4] {
        core::array::from_fn(|i| {
            let mut w = InfoSetWriter(ArrayString::<16>::new());
            w.tantos(tantos).mano(&manos[i], abstracto);
            w.into_inner()
        })
    }

    pub fn with_utility_table(self, utility_table: Rc<[[f64; 40]; 40]>) -> Self {
        Self {
            utility_table: Some(utility_table),
            ..self
        }
    }

    pub fn default_utility_table() -> [[f64; 40]; 40] {
        std::array::from_fn(|t1| std::array::from_fn(|t2| t1 as f64 - t2 as f64))
    }

    fn iter_descartes<const N: usize>(
        &self,
        estado_baraja: [(Carta, u8); 8],
    ) -> Vec<(MusGame, f64)> {
        let mut partidas = Vec::new();
        let mut iter = DistribucionCartaIter::<N, 8>::new(estado_baraja);
        while let Some((nuevas, probability)) = iter.next() {
            let mut game = self.clone();
            game.partida
                .as_mut()
                .expect("Game must exist in descartes phase")
                .descartar_con_nuevas(&nuevas)
                .expect("Game must be expecting a discard but it doesn't");
            let Some(CardSource::Iterable(dist)) = &mut game.cards else {
                unreachable!()
            };
            for (d, f) in std::iter::zip(dist, iter.current_frequencies()) {
                d.1 = *f as u8;
            }
            // Las cartas nuevas cambian la mano de quien descartó.
            game.actualizar_manos(Lance::Grande);
            game.enforce_max_mus_rounds();
            partidas.push((game, probability));
        }
        partidas
    }

    fn first_player_action(&self) -> bool {
        self.partida.as_ref().is_some_and(|partida| {
            matches!(partida.fase(), Some(FasePartida::Envites(_)))
                && matches!(partida.turno(), Some(Turno::Pareja(0 | 1)))
        })
    }

    fn second_player_turn(&self) -> bool {
        self.partida.as_ref().is_some_and(|partida| {
            matches!(partida.fase(), Some(FasePartida::Envites(_)))
                && matches!(partida.turno(), Some(Turno::Pareja(2 | 3)))
        })
    }
}

impl Game for MusGame {
    type Action = Accion;
    const N_PLAYERS: usize = 4;

    fn utility(&mut self, player: usize) -> f64 {
        let tantos = self.partida.as_mut().unwrap().tantos();
        utility(player, &tantos, self.utility_table.as_deref())
    }

    fn info_set_str(&self, player: usize) -> String {
        let mut output = String::with_capacity(15 + self.history_str.len());
        output.push_str(&self.info_set_prefix[player]);
        output.push_str(&self.descarte_str[player]);
        output.push_str(&self.history_str());
        output
    }

    fn new_random(&mut self) {
        let mut reparto_resuelto = false;
        match &mut self.partida {
            None => {
                let mut baraja = Baraja::baraja_mus();
                let manos = baraja.repartir_manos();
                self.set_hands(manos);
                self.set_card_source(CardSource::Baraja(baraja));
            }
            Some(p) => {
                if let Some(CardSource::Baraja(baraja)) = &mut self.cards {
                    let turno = match p
                        .turno()
                        .expect("Some player must be active to call new_random() after game start")
                    {
                        Turno::Jugador(t) => t,
                        Turno::Pareja(_) => unreachable!("Los descartes son individuales."),
                    } as usize;
                    let descartes = p.descartadas().unwrap();
                    InfoSetWriter(&mut self.descarte_str[turno]).descarte(&descartes);
                    let nuevas = baraja.descartar(descartes.into_iter());
                    let _ = p.descartar_con_nuevas(&nuevas);
                    self.history_str.push('C');
                    // Las cartas nuevas cambian la mano de quien descartó.
                    reparto_resuelto = true;
                }
            }
        }
        if reparto_resuelto {
            self.actualizar_manos(Lance::Grande);
        }
        self.enforce_max_mus_rounds();
    }

    fn reset(&mut self) {
        self.partida = None;
        self.history_str.clear();
        self.info_set_prefix = [ArrayString::new(); 4];
        self.descarte_str = [ArrayString::new(); 4];
        self.last_action = None;
        self.manos_pares.clear();
        self.manos_juego.clear();
        self.cards = None;
        self.mus_rounds = 0;
    }

    fn new_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        match &self.partida {
            None => {
                let (tantos, abstract_game, max_mus_rounds) =
                    (self.tantos, self.abstract_game, self.max_mus_rounds);
                let partidas = double_deal(Baraja::FREC_BARAJA_MUS).flat_map(
                    move |(mano1, mano2, probability, dist)| {
                        double_deal(dist).map(move |(mano3, mano4, probability2, dist2)| {
                            let mut game = Self::new(tantos, abstract_game, max_mus_rounds)
                                .with_hands([
                                    Mano::new(mano1),
                                    Mano::new(mano2),
                                    Mano::new(mano3),
                                    Mano::new(mano4),
                                ]);
                            game.set_card_source(CardSource::Iterable(dist2));
                            (game, probability * probability2)
                        })
                    },
                );
                Either::Left(partidas)
            }
            Some(p) => {
                let Some(CardSource::Iterable(estado_baraja)) = self.cards else {
                    todo!()
                };
                let turno = match p
                    .turno()
                    .expect("Some player must be active to call new_iter() after game started")
                {
                    Turno::Jugador(t) => t,
                    Turno::Pareja(_) => unreachable!("Los descartes son individuales."),
                } as usize;
                let mut game = self.clone();
                let descartes = game.partida.as_ref().unwrap().descartadas().unwrap();
                InfoSetWriter(&mut game.descarte_str[turno]).descarte(&descartes);
                game.history_str.push('C');
                let partidas = match descartes.len() {
                    1 => game.iter_descartes::<1>(estado_baraja),
                    2 => game.iter_descartes::<2>(estado_baraja),
                    3 => game.iter_descartes::<3>(estado_baraja),
                    4 => game.iter_descartes::<4>(estado_baraja),
                    _ => unreachable!(),
                };
                Either::Right(partidas.into_iter())
            }
        }
    }

    fn actions(&self) -> Vec<Accion> {
        let partida = self.partida.as_ref().unwrap();
        debug_assert!(
            !matches!(partida.fase(), Some(FasePartida::Mus))
                || self.mus_rounds < self.max_mus_rounds,
            "Mus phase reached and no available rounds: missing call to enforce_max_mus_rounds."
        );
        let mut acciones = actions(partida);
        if self.second_player_turn() {
            // El compañero conoce ya la acción del primer miembro de la pareja y solo puede
            // igualarla o subirla: la acción de la pareja es la suya.
            let last_action = self
                .last_action
                .expect("First player of the couple must act but it's missing");
            acciones.retain(|a| *a >= last_action);
        }
        acciones
    }

    fn current_player(&self) -> NodeType {
        match &self.partida {
            None => NodeType::Chance,
            Some(partida) => match partida.fase() {
                None => NodeType::Terminal,
                Some(FasePartida::DescartePendiente) => NodeType::Chance,
                Some(FasePartida::Mus | FasePartida::Descartes | FasePartida::Envites(_)) => {
                    match partida.turno() {
                        Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                            NodeType::Player(player_id as usize)
                        }
                        None => NodeType::Terminal,
                    }
                }
            },
        }
    }

    fn act(&mut self, a: Accion) {
        self.last_action = Some(a);
        if !self.first_player_action() {
            self.history_str.push_str(&a.to_string());
        }
        let fase = self
            .partida
            .as_ref()
            .expect("At least one PartidaMus must be available.")
            .fase();
        let partida = self.partida.as_mut().unwrap();
        match fase {
            Some(FasePartida::Descartes) => {
                let _ = partida.actuar(a);
            }
            Some(FasePartida::Mus) => {
                // Cada jugador vota por separado, incluidos los dos miembros de una pareja: no
                // hay acción de pareja en la fase de mus.
                let _ = partida.actuar(a);
                match partida.fase() {
                    // Todos han pedido mus: se consume una ronda.
                    Some(FasePartida::Descartes) => self.mus_rounds += 1,
                    Some(FasePartida::Envites(lance)) => self.actualizar_manos(lance),
                    _ => {}
                }
            }
            Some(FasePartida::Envites(lance_previo)) => {
                let _ = partida.actuar(a);
                let Some(FasePartida::Envites(lance_siguiente)) = partida.fase() else {
                    return;
                };
                if lance_previo != lance_siguiente {
                    self.info_set_prefix = MusGame::info_set_prefix(
                        self.partida.as_ref().unwrap().manos(),
                        &self.tantos,
                        self.abstract_game.then_some(lance_siguiente),
                    );
                    push_jugadas_lance(
                        &mut self.history_str,
                        &lance_previo,
                        &lance_siguiente,
                        &self.manos_pares,
                        &self.manos_juego,
                    );
                }
            }
            _ => todo!(),
        }
    }

    fn history_str(&self) -> String {
        if self.second_player_turn() {
            format!("{}{}*", self.history_str, self.last_action.unwrap())
        } else {
            self.history_str.to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct MusGameTwoHands {
    tantos: [u8; 2],
    cards: Option<CardSource>,
    partida: Option<PartidaMus<CuatroJugadores>>,
    history_str: ArrayString<192>,
    info_set_prefix: [ArrayString<24>; 2],
    // Dos manos por jugador, hasta 5 bytes por descarte y ronda.
    descarte_str: [ArrayString<{ 10 * MAX_RONDAS_MUS as usize }>; 2],
    manos_pares: ArrayString<4>,
    manos_juego: ArrayString<4>,
    mus_rounds: u8,
    max_mus_rounds: u8,
    abstract_game: bool,
    utility_table: Option<Rc<[[f64; 40]; 40]>>,
}

impl MusGameTwoHands {
    /// `max_rondas_mus` acota cuántas rondas de mus pueden jugarse, hasta [`MAX_RONDAS_MUS`]. Con
    /// cero rondas la partida se juega a primeras dadas y la fase de mus no forma parte del
    /// árbol de juego.
    pub fn new(tantos: [u8; 2], abstract_game: bool, max_mus_rounds: u8) -> Self {
        assert!(
            max_mus_rounds <= MAX_RONDAS_MUS,
            "El máximo de rondas de mus es {MAX_RONDAS_MUS}, se han pedido {max_mus_rounds}."
        );
        Self {
            partida: None,
            cards: None,
            tantos,
            history_str: ArrayString::new(),
            info_set_prefix: [ArrayString::new(); 2],
            descarte_str: [ArrayString::new(); 2],
            manos_pares: ArrayString::new(),
            manos_juego: ArrayString::new(),
            mus_rounds: 0,
            max_mus_rounds,
            abstract_game,
            utility_table: None,
        }
    }
    pub fn with_hands(self, manos: [Mano; 4]) -> Self {
        let mut new_game = self.clone();
        new_game.set_hands(manos);
        new_game
    }

    fn set_hands(&mut self, manos: [Mano; 4]) {
        self.partida = Some(PartidaMus::<CuatroJugadores>::new(manos, self.tantos));
        self.actualizar_manos(Lance::Grande);
        // La 'M' marca el reparto, no la fase de mus: distingue la partida repartida del nodo
        // de azar inicial, que es el que `GameGraph` indexa con el historial vacío.
        self.history_str = ArrayString::<192>::from("M").unwrap();
        self.enforce_max_mus_rounds();
    }

    /// Sale de la fase de mus cuando se han agotado las rondas configuradas. `NoMus` sería
    /// entonces la única acción posible, así que se fuerza aquí en lugar de crear un nodo de
    /// decisión con una sola opción. Con cero rondas esto ocurre ya en el reparto: se juega a
    /// primeras dadas y la fase de mus no llega a formar parte del árbol.
    ///
    /// Mantiene la invariante de la que depende [`actions`]: si hay un nodo de jugador en
    /// `FasePartida::Mus`, queda al menos una ronda por jugar.
    fn enforce_max_mus_rounds(&mut self) {
        if self.mus_rounds < self.max_mus_rounds {
            return;
        }
        let partida = self
            .partida
            .as_mut()
            .expect("La partida debe estar repartida.");
        if !matches!(partida.fase(), Some(FasePartida::Mus)) {
            return;
        }
        let _ = partida.actuar(Accion::NoMus);
        if let Some(FasePartida::Envites(lance)) = partida.fase() {
            self.actualizar_manos(lance);
        }
    }

    /// Refresca el prefijo del conjunto de información y los indicadores de pares y juego con las
    /// manos que hay ahora sobre la mesa. Se llama al entrar en la fase de envites porque los
    /// descartes pueden haber cambiado las manos repartidas.
    fn actualizar_manos(&mut self, lance: Lance) {
        let manos = self
            .partida
            .as_ref()
            .expect("La partida debe estar repartida.")
            .manos();
        self.info_set_prefix = MusGameTwoHands::info_set_prefix(
            manos,
            &self.tantos,
            self.abstract_game.then_some(lance),
        );
        (self.manos_pares, self.manos_juego) = jugadas_manos(manos);
    }

    fn set_card_source(&mut self, cartas: CardSource) {
        self.cards = Some(cartas);
    }

    fn info_set_prefix(
        manos: &[Mano; 4],
        tantos: &[u8; 2],
        abstracto: Option<Lance>,
    ) -> [ArrayString<24>; 2] {
        core::array::from_fn(|i| {
            if let Some(lance) = abstracto {
                let mano_abstracta = ManosNormalizadas::mano_to_abstract_string(&manos[i], &lance);
                let mano_abstracta2 =
                    ManosNormalizadas::mano_to_abstract_string(&manos[i + 2], &lance);
                ArrayString::from(&format!(
                    "{}:{},{},{},",
                    tantos[0], tantos[1], mano_abstracta, mano_abstracta2
                ))
                .unwrap()
            } else {
                ArrayString::from(&format!(
                    "{}:{},{},{},",
                    tantos[0],
                    tantos[1],
                    manos[i],
                    manos[i + 2]
                ))
                .unwrap()
            }
        })
    }

    pub fn with_utility_table(self, utility_table: Rc<[[f64; 40]; 40]>) -> Self {
        Self {
            utility_table: Some(utility_table),
            ..self
        }
    }

    fn iter_descartes<const N: usize>(
        &self,
        estado_baraja: [(Carta, u8); 8],
    ) -> Vec<(MusGameTwoHands, f64)> {
        let mut partidas = Vec::new();
        let mut iter = DistribucionCartaIter::<N, 8>::new(estado_baraja);
        while let Some((nuevas, probability)) = iter.next() {
            let mut game = self.clone();
            game.partida
                .as_mut()
                .expect("Game must exist in descartes phase")
                .descartar_con_nuevas(&nuevas)
                .expect("Game must be expecting a discard but it doesn't");
            let Some(CardSource::Iterable(dist)) = &mut game.cards else {
                unreachable!()
            };
            for (d, f) in std::iter::zip(dist, iter.current_frequencies()) {
                d.1 = *f as u8;
            }
            // Las cartas nuevas cambian la mano de quien descartó.
            game.actualizar_manos(Lance::Grande);
            game.enforce_max_mus_rounds();
            partidas.push((game, probability));
        }
        partidas
    }
}

impl Game for MusGameTwoHands {
    type Action = Accion;
    const N_PLAYERS: usize = 2;

    fn utility(&mut self, player: usize) -> f64 {
        let tantos = self.partida.as_mut().unwrap().tantos();
        utility(player, &tantos, self.utility_table.as_deref())
    }

    fn info_set_str(&self, player: usize) -> String {
        let mut output = String::with_capacity(15 + self.history_str.len());
        output.push_str(&self.info_set_prefix[player]);
        output.push_str(&self.descarte_str[player]);
        output.push_str(&self.history_str());
        output
    }

    fn new_random(&mut self) {
        let mut reparto_resuelto = false;
        match &mut self.partida {
            None => {
                let mut baraja = Baraja::baraja_mus();
                let manos = baraja.repartir_manos();
                self.set_hands(manos);
                self.set_card_source(CardSource::Baraja(baraja));
            }
            Some(p) => {
                if let Some(CardSource::Baraja(baraja)) = &mut self.cards {
                    let turno = match p
                        .turno()
                        .expect("Some player must be active to call new_random() after game start")
                    {
                        Turno::Jugador(t) => t,
                        Turno::Pareja(_) => todo!(),
                    } as usize;
                    let descartes = p.descartadas().unwrap();
                    InfoSetWriter(&mut self.descarte_str[turno % 2]).descarte(&descartes);
                    let nuevas = baraja.descartar(descartes.into_iter());
                    let _ = p.descartar_con_nuevas(&nuevas);
                    self.history_str.push('C');
                    // Las cartas nuevas cambian la mano de quien descartó.
                    reparto_resuelto = true;
                }
            }
        }
        if reparto_resuelto {
            self.actualizar_manos(Lance::Grande);
        }
        self.enforce_max_mus_rounds();
    }

    fn reset(&mut self) {
        self.partida = None;
        self.history_str.clear();
        self.info_set_prefix = [ArrayString::new(); 2];
        self.manos_pares.clear();
        self.manos_juego.clear();
        self.cards = None;
        self.mus_rounds = 0;
        self.descarte_str = [ArrayString::new(); 2];
    }

    fn new_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        match &self.partida {
            None => {
                let (tantos, abstract_game, max_mus_rounds) =
                    (self.tantos, self.abstract_game, self.max_mus_rounds);
                let partidas = double_deal(Baraja::FREC_BARAJA_MUS).flat_map(
                    move |(mano1, mano2, probability, dist)| {
                        double_deal(dist).map(move |(mano3, mano4, probability2, dist2)| {
                            let mut game = Self::new(tantos, abstract_game, max_mus_rounds)
                                .with_hands([
                                    Mano::new(mano1),
                                    Mano::new(mano2),
                                    Mano::new(mano3),
                                    Mano::new(mano4),
                                ]);
                            game.set_card_source(CardSource::Iterable(dist2));
                            (game, probability * probability2)
                        })
                    },
                );
                Either::Left(partidas)
            }
            Some(p) => {
                let Some(CardSource::Iterable(estado_baraja)) = self.cards else {
                    todo!()
                };
                let turno = match p
                    .turno()
                    .expect("Some player must be active to call new_iter() after game started")
                {
                    Turno::Jugador(t) => t,
                    Turno::Pareja(_) => todo!(),
                } as usize;
                let mut game = self.clone();
                let descartes = game.partida.as_ref().unwrap().descartadas().unwrap();
                InfoSetWriter(&mut game.descarte_str[turno % 2]).descarte(&descartes);
                game.history_str.push('C');
                let games = match descartes.len() {
                    1 => game.iter_descartes::<1>(estado_baraja),
                    2 => game.iter_descartes::<2>(estado_baraja),
                    3 => game.iter_descartes::<3>(estado_baraja),
                    4 => game.iter_descartes::<4>(estado_baraja),
                    _ => unreachable!(),
                };

                Either::Right(games.into_iter())
            }
        }
    }

    fn actions(&self) -> Vec<Accion> {
        let partida = self.partida.as_ref().unwrap();
        debug_assert!(
            !matches!(partida.fase(), Some(FasePartida::Mus))
                || self.mus_rounds < self.max_mus_rounds,
            "Nodo de jugador en la fase de mus sin rondas disponibles:              falta un enforce_max_mus_rounds tras resolver el descarte."
        );
        actions(partida)
    }

    fn current_player(&self) -> NodeType {
        match &self.partida {
            None => NodeType::Chance,
            Some(partida) => match partida.fase() {
                None => NodeType::Terminal,
                Some(FasePartida::DescartePendiente) => NodeType::Chance,
                Some(FasePartida::Mus | FasePartida::Descartes | FasePartida::Envites(_)) => {
                    match partida.turno() {
                        Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                            NodeType::Player(player_id as usize % 2)
                        }
                        None => NodeType::Terminal,
                    }
                }
            },
        }
    }

    fn act(&mut self, a: Accion) {
        self.history_str.push_str(&a.to_string());
        if let Some(partida) = self.partida.as_mut() {
            match &partida.fase() {
                Some(FasePartida::Descartes) => {
                    let _ = partida.actuar(a);
                }
                Some(FasePartida::Mus) => {
                    let _ = partida.actuar(a);
                    match partida.fase() {
                        // Todos han pedido mus: se consume una ronda.
                        Some(FasePartida::Descartes) => self.mus_rounds += 1,
                        Some(FasePartida::Envites(lance)) => self.actualizar_manos(lance),
                        _ => {}
                    }
                }
                Some(FasePartida::Envites(lance_previo)) => {
                    if let Turno::Pareja(_) = partida.turno().expect("some player must be playing")
                    {
                        let _ = partida.actuar(a);
                    }
                    let _ = partida.actuar(a);
                    let Some(FasePartida::Envites(lance_siguiente)) = partida.fase() else {
                        return;
                    };
                    if lance_previo != &lance_siguiente {
                        self.info_set_prefix = MusGameTwoHands::info_set_prefix(
                            self.partida.as_ref().unwrap().manos(),
                            &self.tantos,
                            self.abstract_game.then_some(lance_siguiente),
                        );
                        push_jugadas_lance(
                            &mut self.history_str,
                            lance_previo,
                            &lance_siguiente,
                            &self.manos_pares,
                            &self.manos_juego,
                        );
                    }
                }
                _ => todo!(),
            }
        }
    }

    fn history_str(&self) -> String {
        self.history_str.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct MusGameTwoPlayers {
    tantos: [u8; 2],
    cards: Option<CardSource>,
    partida: Option<PartidaMus<DosJugadores>>,
    history_str: ArrayString<192>,
    info_set_prefix: [ArrayString<16>; 2],
    // Una mano por jugador, hasta 5 bytes por descarte y ronda.
    descarte_str: [ArrayString<{ 5 * MAX_RONDAS_MUS as usize }>; 2],
    manos_pares: ArrayString<4>,
    manos_juego: ArrayString<4>,
    mus_rounds: u8,
    max_mus_rounds: u8,
    abstract_game: bool,
    utility_table: Option<Rc<[[f64; 40]; 40]>>,
}

impl MusGameTwoPlayers {
    /// `max_rondas_mus` acota cuántas rondas de mus pueden jugarse, hasta [`MAX_RONDAS_MUS`]. Con
    /// cero rondas la partida se juega a primeras dadas y la fase de mus no forma parte del
    /// árbol de juego.
    pub fn new(tantos: [u8; 2], abstract_game: bool, max_mus_rounds: u8) -> Self {
        assert!(
            max_mus_rounds <= MAX_RONDAS_MUS,
            "El máximo de rondas de mus es {MAX_RONDAS_MUS}, se han pedido {max_mus_rounds}."
        );
        Self {
            partida: None,
            cards: None,
            tantos,
            history_str: ArrayString::new(),
            info_set_prefix: [ArrayString::new(); 2],
            descarte_str: [ArrayString::new(); 2],
            manos_pares: ArrayString::new(),
            manos_juego: ArrayString::new(),
            mus_rounds: 0,
            max_mus_rounds,
            abstract_game,
            utility_table: None,
        }
    }

    pub fn with_utility_table(self, utility_table: Rc<[[f64; 40]; 40]>) -> Self {
        Self {
            utility_table: Some(utility_table),
            ..self
        }
    }

    pub fn with_hands(self, manos: [Mano; 2]) -> Self {
        let mut new_game = self.clone();
        new_game.set_hands(manos);
        new_game
    }

    fn set_hands(&mut self, manos: [Mano; 2]) {
        self.partida = Some(PartidaMus::<DosJugadores>::new(manos, self.tantos));
        self.actualizar_manos(Lance::Grande);
        // La 'M' marca el reparto, no la fase de mus: distingue la partida repartida del nodo
        // de azar inicial, que es el que `GameGraph` indexa con el historial vacío.
        self.history_str = ArrayString::<192>::from("M").unwrap();
        self.enforce_max_mus_rounds();
    }

    /// Sale de la fase de mus cuando se han agotado las rondas configuradas. `NoMus` sería
    /// entonces la única acción posible, así que se fuerza aquí en lugar de crear un nodo de
    /// decisión con una sola opción. Con cero rondas esto ocurre ya en el reparto: se juega a
    /// primeras dadas y la fase de mus no llega a formar parte del árbol.
    ///
    /// Mantiene la invariante de la que depende [`actions`]: si hay un nodo de jugador en
    /// `FasePartida::Mus`, queda al menos una ronda por jugar.
    fn enforce_max_mus_rounds(&mut self) {
        if self.mus_rounds < self.max_mus_rounds {
            return;
        }
        let partida = self
            .partida
            .as_mut()
            .expect("La partida debe estar repartida.");
        if !matches!(partida.fase(), Some(FasePartida::Mus)) {
            return;
        }
        let _ = partida.actuar(Accion::NoMus);
        if let Some(FasePartida::Envites(lance)) = partida.fase() {
            self.actualizar_manos(lance);
        }
    }

    /// Refresca el prefijo del conjunto de información y los indicadores de pares y juego con las
    /// manos que hay ahora sobre la mesa. Se llama al entrar en la fase de envites porque los
    /// descartes pueden haber cambiado las manos repartidas.
    fn actualizar_manos(&mut self, lance: Lance) {
        let manos = self
            .partida
            .as_ref()
            .expect("La partida debe estar repartida.")
            .manos();
        self.info_set_prefix = MusGameTwoPlayers::info_set_prefix(
            manos,
            &self.tantos,
            self.abstract_game.then_some(lance),
        );
        (self.manos_pares, self.manos_juego) = jugadas_manos(manos);
    }

    fn set_card_source(&mut self, cartas: CardSource) {
        self.cards = Some(cartas);
    }

    pub fn mus_game(&self) -> Option<&PartidaMus<DosJugadores>> {
        self.partida.as_ref()
    }

    fn info_set_prefix(
        manos: &[Mano; 2],
        tantos: &[u8; 2],
        abstracto: Option<Lance>,
    ) -> [ArrayString<16>; 2] {
        core::array::from_fn(|i| {
            let mut w = InfoSetWriter(ArrayString::<16>::new());
            w.tantos(tantos).mano(&manos[i], abstracto);
            w.into_inner()
        })
    }

    fn iter_descartes<const N: usize>(
        &self,
        estado_baraja: [(Carta, u8); 8],
    ) -> Vec<(MusGameTwoPlayers, f64)> {
        let mut partidas = Vec::new();
        let mut iter = DistribucionCartaIter::<N, 8>::new(estado_baraja);
        while let Some((nuevas, probability)) = iter.next() {
            let mut game = self.clone();
            game.partida
                .as_mut()
                .expect("Game must exist in descartes phase")
                .descartar_con_nuevas(&nuevas)
                .expect("Game must be expecting a discard but it doesn't");
            let Some(CardSource::Iterable(dist)) = &mut game.cards else {
                unreachable!()
            };
            for (d, f) in std::iter::zip(dist, iter.current_frequencies()) {
                d.1 = *f as u8;
            }
            // Las cartas nuevas cambian la mano de quien descartó.
            game.actualizar_manos(Lance::Grande);
            game.enforce_max_mus_rounds();
            partidas.push((game, probability));
        }
        partidas
    }
}

impl Game for MusGameTwoPlayers {
    type Action = Accion;
    const N_PLAYERS: usize = 2;

    fn utility(&mut self, player: usize) -> f64 {
        let tantos = self.partida.as_mut().unwrap().tantos();
        utility(player, &tantos, self.utility_table.as_deref())
    }

    fn info_set_str(&self, player: usize) -> String {
        let mut output = String::with_capacity(15 + self.history_str.len());
        output.push_str(&self.info_set_prefix[player]);
        output.push_str(&self.descarte_str[player]);
        output.push_str(&self.history_str());
        output
    }

    fn new_random(&mut self) {
        let mut reparto_resuelto = false;
        match &mut self.partida {
            None => {
                let mut baraja = Baraja::baraja_mus();
                let manos = baraja.repartir_manos();
                self.set_hands(manos);
                self.set_card_source(CardSource::Baraja(baraja));
            }
            Some(p) => {
                if let Some(CardSource::Baraja(baraja)) = &mut self.cards {
                    let turno = match p
                        .turno()
                        .expect("Some player must be active to call new_random() after game start")
                    {
                        Turno::Jugador(t) => t,
                        Turno::Pareja(_) => todo!(),
                    } as usize;
                    let descartes = p.descartadas().unwrap();
                    InfoSetWriter(&mut self.descarte_str[turno]).descarte(&descartes);
                    let nuevas = baraja.descartar(descartes.into_iter());
                    let _ = p.descartar_con_nuevas(&nuevas);
                    self.history_str.push('C');
                    // Las cartas nuevas cambian la mano de quien descartó.
                    reparto_resuelto = true;
                }
            }
        }
        if reparto_resuelto {
            self.actualizar_manos(Lance::Grande);
        }
        self.enforce_max_mus_rounds();
    }

    fn reset(&mut self) {
        self.partida = None;
        self.history_str.clear();
        self.info_set_prefix = [ArrayString::new(); 2];
        self.manos_pares.clear();
        self.manos_juego.clear();
        self.cards = None;
        self.mus_rounds = 0;
        self.descarte_str = [ArrayString::new(); 2];
    }

    fn new_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        match &self.partida {
            None => {
                let (tantos, abstract_game, max_mus_rounds) =
                    (self.tantos, self.abstract_game, self.max_mus_rounds);
                let partidas = double_deal(Baraja::FREC_BARAJA_MUS).map(
                    move |(mano1, mano2, probability, dist)| {
                        let mut game = Self::new(tantos, abstract_game, max_mus_rounds)
                            .with_hands([Mano::new(mano1), Mano::new(mano2)]);
                        game.set_card_source(CardSource::Iterable(dist));
                        (game, probability)
                    },
                );
                Either::Left(partidas)
            }
            Some(p) => {
                let Some(CardSource::Iterable(estado_baraja)) = self.cards else {
                    todo!()
                };
                let turno = match p
                    .turno()
                    .expect("Some player must be active to call new_iter() after game started")
                {
                    Turno::Jugador(t) => t,
                    Turno::Pareja(_) => todo!(),
                } as usize;
                let mut game = self.clone();
                let descartes = game.partida.as_ref().unwrap().descartadas().unwrap();
                InfoSetWriter(&mut game.descarte_str[turno]).descarte(&descartes);
                game.history_str.push('C');
                let games = match descartes.len() {
                    1 => game.iter_descartes::<1>(estado_baraja),
                    2 => game.iter_descartes::<2>(estado_baraja),
                    3 => game.iter_descartes::<3>(estado_baraja),
                    4 => game.iter_descartes::<4>(estado_baraja),
                    _ => unreachable!(),
                };
                Either::Right(games.into_iter())
            }
        }
    }

    fn actions(&self) -> Vec<Accion> {
        let partida = self.partida.as_ref().unwrap();
        debug_assert!(
            !matches!(partida.fase(), Some(FasePartida::Mus))
                || self.mus_rounds < self.max_mus_rounds,
            "Mus phase reached and no available rounds: missing call to enforce_max_mus_rounds."
        );
        actions(partida)
    }

    fn current_player(&self) -> NodeType {
        match &self.partida {
            None => NodeType::Chance,
            Some(partida) => match partida.fase() {
                None => NodeType::Terminal,
                Some(FasePartida::DescartePendiente) => NodeType::Chance,
                Some(FasePartida::Mus | FasePartida::Descartes | FasePartida::Envites(_)) => {
                    match partida.turno() {
                        Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                            NodeType::Player(player_id as usize % 2)
                        }
                        None => NodeType::Terminal,
                    }
                }
            },
        }
    }

    fn act(&mut self, a: Accion) {
        self.history_str.push_str(&a.to_string());
        if let Some(partida) = self.partida.as_mut() {
            match &partida.fase() {
                Some(FasePartida::Descartes) => {
                    let _ = partida.actuar(a);
                }
                Some(FasePartida::Mus) => {
                    let _ = partida.actuar(a);
                    match partida.fase() {
                        // Todos han pedido mus: se consume una ronda.
                        Some(FasePartida::Descartes) => self.mus_rounds += 1,
                        Some(FasePartida::Envites(lance)) => self.actualizar_manos(lance),
                        _ => {}
                    }
                }
                Some(FasePartida::Envites(lance_previo)) => {
                    let _ = partida.actuar(a);
                    let Some(FasePartida::Envites(lance_siguiente)) = partida.fase() else {
                        return;
                    };
                    if lance_previo != &lance_siguiente {
                        self.info_set_prefix = MusGameTwoPlayers::info_set_prefix(
                            self.partida.as_ref().unwrap().manos(),
                            &self.tantos,
                            self.abstract_game.then_some(lance_siguiente),
                        );
                        push_jugadas_lance(
                            &mut self.history_str,
                            lance_previo,
                            &lance_siguiente,
                            &self.manos_pares,
                            &self.manos_juego,
                        );
                    }
                }
                _ => todo!(),
            }
        }
    }

    fn history_str(&self) -> String {
        self.history_str.to_string()
    }
}

/// Acciones legales en el estado actual. Solo se llega a un nodo de jugador en la fase de mus
/// mientras queden rondas por jugar: al agotarse, `salir_de_mus_si_agotado` fuerza el `NoMus` en
/// vez de dejar aquí un nodo con una única acción.
fn actions<T: ModalidadMus>(partida: &PartidaMus<T>) -> Vec<Accion> {
    match partida.fase() {
        Some(FasePartida::Mus) => {
            vec![Accion::Mus, Accion::NoMus]
        }
        Some(FasePartida::Descartes) => {
            let turno = match partida
                .turno()
                .expect("Some player must be active to call actions()")
            {
                Turno::Jugador(t) => t,
                Turno::Pareja(t) => t,
            } as usize;
            let mano = &partida.manos().as_ref()[turno];
            let mut descartes = [false; 4];
            for (idx, carta) in mano.iter().enumerate() {
                descartes[idx] = *carta != Carta::Rey;
            }
            if descartes == [false; 4] {
                descartes[0] = true;
            }
            vec![Accion::Descartar(descartes)]
        }
        Some(FasePartida::Envites(_)) => {
            let fase_envites = partida.fase_envites().unwrap();
            let ultimo_envite: Apuesta = fase_envites.ultima_apuesta();
            let apuesta_maxima = fase_envites.apuesta_maxima();
            let mut actions = match ultimo_envite {
                Apuesta::Tantos(tantos) if tantos == apuesta_maxima => {
                    return vec![Accion::Paso, Accion::Quiero, Accion::Ordago];
                }
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
                Apuesta::Ordago => return vec![Accion::Paso, Accion::Quiero],
                _ => return vec![Accion::Paso, Accion::Quiero, Accion::Ordago],
            };
            actions.retain(|action| {
                if let Apuesta::Tantos(tantos) = ultimo_envite
                    && let Accion::Envido(v) = action
                {
                    tantos + *v < apuesta_maxima
                } else {
                    true
                }
            });
            actions
        }
        Some(FasePartida::DescartePendiente) => vec![],
        None => todo!(),
    }
}

fn utility(player: usize, tantos: &[u8; 2], utility_table: Option<&[[f64; 40]; 40]>) -> f64 {
    utility_table.as_ref().map_or_else(
        || {
            let payoff = [
                tantos[0] as i8 - tantos[1] as i8,
                tantos[1] as i8 - tantos[0] as i8,
            ];

            payoff[player % 2] as f64
        },
        |utility_table| {
            if tantos[0] == 40 || tantos[1] == 40 {
                let payoff = [
                    tantos[0] as i8 - tantos[1] as i8,
                    tantos[1] as i8 - tantos[0] as i8,
                ];

                payoff[player % 2] as f64
            } else {
                let expected_utility = utility_table[tantos[1] as usize][tantos[0] as usize];
                if player.is_multiple_of(2) {
                    -expected_utility
                } else {
                    expected_utility
                }
            }
        },
    )
}

/// Añade al historial los indicadores de pares y juego cuando el lance en curso cambia. Son
/// información pública: al llegar a pares se sabe qué manos tienen pares, y al llegar a juego o
/// punto, además, cuáles tienen juego.
fn push_jugadas_lance(
    history_str: &mut ArrayString<192>,
    lance_previo: &Lance,
    lance_siguiente: &Lance,
    manos_pares: &ArrayString<4>,
    manos_juego: &ArrayString<4>,
) {
    match lance_siguiente {
        Lance::Pares => history_str.push_str(manos_pares),
        Lance::Punto | Lance::Juego => {
            if lance_previo != &Lance::Pares {
                history_str.push_str(manos_pares);
            }
            history_str.push_str(manos_juego);
        }
        _ => {}
    }
}

type DoubleDeal = ([Carta; 4], [Carta; 4], f64, [(Carta, u8); 8]);

fn double_deal(cartas: [(Carta, u8); 8]) -> impl Iterator<Item = DoubleDeal> {
    struct DoubleDealIter(DistribucionDobleCartaIter<4, 8>);

    impl Iterator for DoubleDealIter {
        type Item = DoubleDeal;

        fn next(&mut self) -> Option<Self::Item> {
            let (mano1, mano2, prob) = self.0.next()?;

            let mut dist = self.0.cartas();
            for (d, f) in std::iter::zip(&mut dist, self.0.current_frequencies()) {
                d.1 = *f as u8;
            }

            Some((mano1, mano2, prob, dist))
        }
    }
    DoubleDealIter(DistribucionDobleCartaIter::new(cartas))
}

fn jugadas_manos(manos: &[Mano]) -> (ArrayString<4>, ArrayString<4>) {
    let manos_pares = manos
        .iter()
        .map(|m| if m.pares().is_some() { '1' } else { '0' })
        .join("");
    let manos_juego = manos
        .iter()
        .map(|m| if m.juego().is_some() { '1' } else { '0' })
        .join("");

    (
        ArrayString::from(&manos_pares).unwrap(),
        ArrayString::from(&manos_juego).unwrap(),
    )
}

#[derive(Debug, Clone)]
enum CardSource {
    Baraja(Baraja),
    Iterable([(Carta, u8); 8]),
}

struct InfoSetWriter<W: Write>(W);

impl<W: Write> InfoSetWriter<W> {
    fn tantos(&mut self, tantos: &[u8; 2]) -> &mut Self {
        let _ = write!(self.0, "{}:{},", tantos[0], tantos[1]);
        self
    }

    fn mano(&mut self, mano: &Mano, abstracto: Option<Lance>) -> &mut Self {
        if let Some(lance) = abstracto {
            let mano_abstracta = ManosNormalizadas::mano_to_abstract_string(mano, &lance);
            let _ = write!(self.0, "{},", mano_abstracta);

            self
        } else {
            let _ = write!(self.0, "{},", mano);

            self
        }
    }

    fn descarte(&mut self, descartes: &[Carta]) -> &mut Self {
        for d in descartes {
            let _ = write!(self.0, "{}", char::from(d));
        }
        let _ = write!(self.0, ",");

        self
    }

    fn into_inner(self) -> W {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn two_players_infoset() {
        let manos = [
            Mano::from_str("RRR5").unwrap(),
            Mano::from_str("RCC1").unwrap(),
        ];
        let mut game = MusGameTwoPlayers::new([38, 37], false, 1).with_hands(manos);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,M");
        game.act(Accion::NoMus);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mn");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(1), "38:37,RCC1,Mnp");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpp");
        game.act(Accion::Paso);
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpppo");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpppop11");
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(1), "38:37,RCC1,Mnpppop11o");
        game.act(Accion::Paso);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpppop11op11");

        let manos = [
            Mano::from_str("R775").unwrap(),
            Mano::from_str("S651").unwrap(),
        ];
        let mut game = MusGameTwoPlayers::new([38, 37], false, 1).with_hands(manos);
        game.act(Accion::NoMus);
        game.act(Accion::Paso);
        game.act(Accion::Paso);

        game.act(Accion::Paso);
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(0), "38:37,R775,Mnpppo");
        game.act(Accion::Paso);

        assert_eq!(game.info_set_str(0), "38:37,R775,Mnpppop1000");
    }

    #[test]
    fn two_players_actions() {
        let manos = [
            Mano::from_str("RRR5").unwrap(),
            Mano::from_str("RCC1").unwrap(),
        ];
        let mut game = MusGameTwoPlayers::new([35, 35], false, 1).with_hands(manos.clone());
        game.act(Accion::NoMus);
        game.act(Accion::Envido(2));
        game.act(Accion::Envido(2));
        assert_eq!(
            game.actions(),
            vec![Accion::Paso, Accion::Quiero, Accion::Ordago,]
        );
        game.act(Accion::Quiero);
        game.act(Accion::Envido(10));
        assert_eq!(
            game.actions(),
            vec![Accion::Paso, Accion::Quiero, Accion::Ordago,]
        );
        let mut game = MusGameTwoPlayers::new([37, 37], false, 1).with_hands(manos.clone());
        game.act(Accion::NoMus);
        assert_eq!(
            game.actions(),
            vec![Accion::Paso, Accion::Envido(2), Accion::Ordago,]
        );
        game.act(Accion::Envido(2));
        assert_eq!(
            game.actions(),
            vec![Accion::Paso, Accion::Quiero, Accion::Ordago,]
        );
        game.act(Accion::Paso);
        assert_eq!(
            game.actions(),
            vec![Accion::Paso, Accion::Envido(2), Accion::Ordago,]
        );
        game.act(Accion::Paso);
        game.act(Accion::Envido(2));
        game.act(Accion::Paso);
        assert_eq!(game.actions(), vec![Accion::Paso, Accion::Ordago]);
    }

    /// Cuatro manos con pares y juego, para que todos los lances se jueguen con las dos parejas
    /// completas.
    fn cuatro_manos() -> [Mano; 4] {
        [
            Mano::from_str("RRR5").unwrap(),
            Mano::from_str("RCC1").unwrap(),
            Mano::from_str("RRR4").unwrap(),
            Mano::from_str("RCC7").unwrap(),
        ]
    }

    #[test]
    fn two_hands_infoset() {
        let mut game = MusGameTwoHands::new([38, 37], false, 1).with_hands(cuatro_manos());
        // Cada jugador ve las dos manos de su pareja.
        assert_eq!(game.info_set_str(0), "38:37,RRR5,RRR4,M");
        assert_eq!(game.info_set_str(1), "38:37,RCC1,RCC7,M");

        // Fase de mus: los cuatro puestos votan por separado, aunque cada jugador controle dos.
        assert_eq!(game.current_player(), NodeType::Player(0));
        game.act(Accion::Mus);
        assert_eq!(game.current_player(), NodeType::Player(0));
        assert_eq!(game.info_set_str(0), "38:37,RRR5,RRR4,Mm");
        game.act(Accion::Mus);
        assert_eq!(game.current_player(), NodeType::Player(1));
        assert_eq!(game.info_set_str(1), "38:37,RCC1,RCC7,Mmm");
        game.act(Accion::NoMus);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,RRR4,Mmmn");

        // Grande. Una sola acción por pareja: el jugador actúa por sus dos manos.
        assert_eq!(game.current_player(), NodeType::Player(0));
        game.act(Accion::Paso);
        assert_eq!(game.current_player(), NodeType::Player(1));
        assert_eq!(game.info_set_str(1), "38:37,RCC1,RCC7,Mmmnp");
        game.act(Accion::Paso);
        // Chica
        game.act(Accion::Paso);
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,RRR4,Mmmnpppo");
        game.act(Accion::Paso);
        // Pares: se hace público qué manos tienen pares.
        assert_eq!(game.info_set_str(0), "38:37,RRR5,RRR4,Mmmnpppop1111");
        game.act(Accion::Ordago);
        game.act(Accion::Paso);
        // Juego: se hace público qué manos tienen juego.
        assert_eq!(game.info_set_str(1), "38:37,RCC1,RCC7,Mmmnpppop1111op1111");
    }

    #[test]
    fn mus_game_infoset() {
        let mut game = MusGame::new([38, 37], false, 1).with_hands(cuatro_manos());
        // Cada jugador ve únicamente su mano.
        assert_eq!(game.info_set_str(0), "38:37,RRR5,M");
        assert_eq!(game.info_set_str(1), "38:37,RCC1,M");
        assert_eq!(game.info_set_str(2), "38:37,RRR4,M");
        assert_eq!(game.info_set_str(3), "38:37,RCC7,M");

        assert_eq!(game.current_player(), NodeType::Player(0));
        game.act(Accion::NoMus);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mn");

        // Grande. El primer miembro de la pareja actúa a ciegas: su acción solo la ve su
        // compañero, marcada con un asterisco, y no llega al historial hasta que este responde.
        game.act(Accion::Paso);
        assert_eq!(game.current_player(), NodeType::Player(2));
        assert_eq!(game.info_set_str(2), "38:37,RRR4,Mnp*");
        game.act(Accion::Paso);
        assert_eq!(game.current_player(), NodeType::Player(1));
        assert_eq!(game.info_set_str(1), "38:37,RCC1,Mnp");
        game.act(Accion::Paso);
        game.act(Accion::Paso);
        // Chica
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpp");
        game.act(Accion::Paso);
        game.act(Accion::Paso);
        game.act(Accion::Ordago);
        // El compañero ya conoce el órdago y solo puede igualarlo o subirlo.
        assert_eq!(game.current_player(), NodeType::Player(3));
        assert_eq!(game.info_set_str(3), "38:37,RCC7,Mnpppo*");
        assert_eq!(game.actions(), vec![Accion::Ordago]);
        game.act(Accion::Ordago);
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpppo");
        game.act(Accion::Paso);
        game.act(Accion::Paso);
        // Pares: se hace público qué manos tienen pares.
        assert_eq!(game.info_set_str(0), "38:37,RRR5,Mnpppop1111");
        game.act(Accion::Ordago);
        game.act(Accion::Ordago);
        game.act(Accion::Paso);
        game.act(Accion::Paso);
        // Juego: se hace público qué manos tienen juego.
        assert_eq!(game.info_set_str(2), "38:37,RRR4,Mnpppop1111op1111");
    }

    #[test]
    fn mus_game_descartes_infoset() {
        let mut game = MusGame::new([0, 0], false, 1).with_hands(cuatro_manos());
        // La distribución de la que salen las cartas nuevas; el test solo depende de las cartas
        // descartadas, que son las que entran en el conjunto de información.
        game.set_card_source(CardSource::Iterable(Baraja::FREC_BARAJA_MUS));

        for jugador in [0, 2, 1, 3] {
            assert_eq!(game.current_player(), NodeType::Player(jugador));
            assert_eq!(game.actions(), vec![Accion::Mus, Accion::NoMus]);
            game.act(Accion::Mus);
        }
        assert_eq!(game.info_set_str(0), "0:0,RRR5,Mmmmm");

        // Descartes: el jugador mano se queda con los tres reyes.
        assert_eq!(game.current_player(), NodeType::Player(0));
        let descarte = game.actions()[0];
        assert_eq!(descarte, Accion::Descartar([false, false, false, true]));
        game.act(descarte);

        // Nodo de azar: reparte las cartas nuevas.
        assert_eq!(game.current_player(), NodeType::Chance);
        let (game_tras_descarte, _) = game.new_iter().next().unwrap();
        game = game_tras_descarte;

        // Las cartas descartadas son privadas y la mano del prefijo ya es la nueva.
        let mano_nueva = game.partida.as_ref().unwrap().manos()[0].to_string();
        assert_eq!(game.info_set_str(0), format!("0:0,{mano_nueva},5,Mmmmmd1C"));
        assert_eq!(game.info_set_str(1), "0:0,RCC1,Mmmmmd1C");

        for _ in 0..3 {
            let descarte = game.actions()[0];
            game.act(descarte);
            let (siguiente, _) = game.new_iter().next().unwrap();
            game = siguiente;
        }

        // Agotada la única ronda de mus, la partida entra en la fase de envites sin pasar por un
        // nodo de decisión, y tanto el prefijo como los indicadores de pares y juego se refieren
        // ya a las manos posteriores al descarte.
        assert_eq!(
            game.partida.as_ref().unwrap().fase(),
            Some(FasePartida::Envites(Lance::Grande))
        );
        let manos_reales = game.partida.as_ref().unwrap().manos().clone();
        assert_eq!(
            game.info_set_prefix,
            MusGame::info_set_prefix(&manos_reales, &game.tantos, None)
        );
        assert_eq!(
            (game.manos_pares, game.manos_juego),
            jugadas_manos(&manos_reales)
        );
        assert_eq!(game.history_str(), "Mmmmmd1Cd3Cd1Cd3C");
        // Cada jugador solo conoce sus propios descartes.
        assert!(game.info_set_str(1).contains(",CC1,"));
        assert!(!game.info_set_str(1).contains(",5,"));
    }

    #[test]
    fn update_hands_after_discard() {
        // Ninguna mano tiene juego al reparto inicial.
        let manos = [
            Mano::from_str("4411").unwrap(),
            Mano::from_str("5511").unwrap(),
        ];
        let mut game = MusGameTwoPlayers::new([0, 0], false, 1).with_hands(manos);
        assert_eq!(game.manos_juego.as_str(), "00");

        game.act(Accion::Mus);
        game.act(Accion::Mus);

        game.act(Accion::Descartar([true, true, true, false]));
        let _ = game.partida.as_mut().unwrap().descartar_con_nuevas(&[
            Carta::Rey,
            Carta::Rey,
            Carta::Rey,
        ]);
        game.act(Accion::Descartar([true, false, false, false]));
        let _ = game
            .partida
            .as_mut()
            .unwrap()
            .descartar_con_nuevas(&[Carta::As]);

        game.act(Accion::NoMus);

        let manos_reales = game.partida.as_ref().unwrap().manos().clone();
        assert!(manos_reales[0].hay_juego());
        assert!(!manos_reales[1].hay_juego());

        // El flag de juego debe reflejar las manos tras el descarte, no el reparto inicial.
        assert_eq!(game.manos_juego.as_str(), "10");
        let prefijo_esperado =
            MusGameTwoPlayers::info_set_prefix(&manos_reales, &game.tantos, None);
        assert_eq!(game.info_set_prefix, prefijo_esperado);
    }
}
