use std::collections::{BTreeSet, HashMap};
use std::{rc::Rc, sync::Arc};

use arrayvec::ArrayVec;
use itertools::Either;

use crate::{
    Game, NodeType,
    mus::{
        Accion, Apuesta, Baraja, Carta, CartaIter, CuatroJugadores, DosJugadores, EstadoLance,
        FasePartida, Lance, Mano, ModalidadMus, PartidaMus, RepartoDescarteMusIter,
        RepartoMusDosJugadoresIter, RepartoMusIter, Turno,
    },
    solver::AbstractJugada,
};

/// Maximum number of rounds of the mus phase in [`MusGame`], [`MusGameTwoHands`] and
/// [`MusGameTwoPlayers`].
pub const MAX_RONDAS_MUS: u8 = 1;

#[derive(Debug, Clone)]
pub struct MusGame {
    tantos: [u8; 2],
    cards: Option<CardSource>,
    partida: Option<PartidaMus<CuatroJugadores>>,
    mus_rounds: u8,
    max_mus_rounds: u8,
    abstract_game: bool,
    info_set_builder: MusInfoSetBuilder,
    /// Acción del primer miembro de la pareja, pendiente de que responda el compañero. Solo se
    /// usa para restringir las acciones legales del segundo miembro en [`MusGame::actions`].
    last_action: Option<Accion>,
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
            mus_rounds: 0,
            max_mus_rounds,
            abstract_game,
            info_set_builder: MusInfoSetBuilder::new(abstract_game),
            last_action: None,
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
        self.update_hands(Lance::Grande);
        self.info_set_builder.begin_mus();
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
            // El mus se salta sin pasar por `act`, así que hay que abrir aquí la secuencia de
            // apuestas del primer lance igual que haría la transición Mus -> Envites.
            self.info_set_builder.begin_lance(&lance);
            self.update_hands(lance);
        }
    }

    /// Refresca las manos del conjunto de información con las que hay ahora sobre la mesa. Se
    /// llama al entrar en la fase de envites porque los descartes pueden haber cambiado las manos
    /// repartidas.
    fn update_hands(&mut self, lance: Lance) {
        let manos = self
            .partida
            .as_ref()
            .expect("La partida debe estar repartida.")
            .manos();
        for (player_idx, mano) in manos.iter().enumerate() {
            self.info_set_builder.set_hand(player_idx, mano, &lance);
        }
    }

    fn set_card_source(&mut self, cartas: CardSource) {
        self.cards = Some(cartas);
    }

    pub fn mus_game(&self) -> Option<&PartidaMus<CuatroJugadores>> {
        self.partida.as_ref()
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

    fn iter_descartes<const N: usize>(game: Self) -> impl Iterator<Item = (Self, f64)> {
        let Some(CardSource::Iterable(estado_baraja)) = game.cards else {
            panic!("iter_descartes expects an iterable CardSource");
        };
        let iter = RepartoDescarteMusIter::<N>::new(estado_baraja);
        iter.map(move |(nuevas, probability, dist)| {
            let mut new_game = game.clone();
            new_game
                .partida
                .as_mut()
                .expect("Game must exist in descartes phase")
                .descartar_con_nuevas(&nuevas)
                .expect("Game must be expecting a discard but it doesn't");
            new_game.set_card_source(CardSource::Iterable(dist));
            // Las cartas nuevas cambian la mano de quien descartó.
            new_game.update_hands(Lance::Grande);
            new_game.enforce_max_mus_rounds();
            (new_game, probability)
        })
    }

    fn second_player_turn(&self) -> bool {
        self.partida.as_ref().is_some_and(|partida| {
            matches!(partida.fase(), Some(FasePartida::Envites(_)))
                && matches!(partida.turno(), Some(Turno::Pareja(2 | 3)))
        })
    }

    pub fn actions(&self) -> ArrayVec<Accion, 6> {
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

    pub fn act_with_action(&mut self, action: Accion) {
        self.last_action = Some(action);
        let (turno, phase, new_phase) = {
            let partida = self
                .partida
                .as_mut()
                .expect("partida must be initialized before calling act");
            let phase = partida.fase();
            let turno = partida.turno().expect("some player must be active");
            let _ = partida.actuar(action);
            let new_phase = partida.fase();
            (turno, phase, new_phase)
        };
        let is_first_partner = matches!(turno, Turno::Pareja(0 | 1));
        match phase {
            Some(FasePartida::Mus) => {
                if is_first_partner && action != Accion::NoMus {
                    self.info_set_builder.set_hidden_action(Some(action));
                } else {
                    // Un NoMus del primer miembro corta el mus de inmediato: cierra la decisión de
                    // la pareja sin que el compañero llegue a votar.
                    self.info_set_builder.step_mus(action);
                    self.info_set_builder.set_hidden_action(None);
                }
            }
            Some(FasePartida::Envites(_)) => {
                if is_first_partner {
                    self.info_set_builder.set_hidden_action(Some(action));
                } else {
                    self.info_set_builder.step_lance(action);
                    self.info_set_builder.set_hidden_action(None);
                }
            }
            _ => {}
        }
        match (phase, new_phase) {
            (Some(FasePartida::Mus), Some(FasePartida::Descartes)) => {
                self.info_set_builder.begin_descartes();
                self.mus_rounds += 1;
            }
            (Some(FasePartida::Descartes), Some(FasePartida::Mus)) => {
                self.info_set_builder.begin_mus();
            }
            (Some(FasePartida::Descartes), Some(FasePartida::DescartePendiente)) => {
                let descartes = self.partida.as_ref().unwrap().descartadas().unwrap();
                self.info_set_builder
                    .set_descartes(turno.player_id() as usize, &descartes);
            }
            (Some(FasePartida::Mus), Some(FasePartida::Envites(lance))) => {
                self.info_set_builder.begin_lance(&lance);
                self.update_hands(lance);
            }
            (
                Some(FasePartida::Envites(lance_previo)),
                Some(FasePartida::Envites(lance_siguiente)),
            ) if lance_previo != lance_siguiente => {
                self.info_set_builder.begin_lance(&lance_siguiente);
                self.update_hands(lance_siguiente);
                let manos = self
                    .partida
                    .as_ref()
                    .expect("partida must be initialized if phase is envites")
                    .manos();
                self.info_set_builder.set_jugada(&lance_siguiente, manos);
            }
            _ => {}
        }
    }
}

impl Game for MusGame {
    type InfoSet = MusInfoSet;
    const N_PLAYERS: usize = 4;

    fn utility(&self, player: usize) -> f64 {
        let tantos = self.partida.as_ref().unwrap().tantos();
        utility(player, &tantos, self.utility_table.as_deref())
    }

    fn info_set(&self, player: usize) -> Self::InfoSet {
        self.info_set_builder.to_mus_infoset(player)
    }

    fn chance_sample(&self) -> Self {
        let mut new_game = self.clone();
        match &mut new_game.partida {
            None => {
                let mut baraja = Baraja::baraja_mus();
                let manos = baraja.repartir_manos();
                new_game.set_hands(manos);
                new_game.set_card_source(CardSource::Baraja(baraja));
            }
            Some(p) => {
                if let Some(CardSource::Baraja(baraja)) = &mut new_game.cards {
                    let descartes = p.descartadas().unwrap();
                    let nuevas = baraja.descartar(descartes.into_iter());
                    let _ = p.descartar_con_nuevas(&nuevas);
                    // Las cartas nuevas cambian la mano de quien descartó.
                    new_game.update_hands(Lance::Grande);
                }
            }
        }
        new_game.enforce_max_mus_rounds();

        new_game
    }

    fn chance_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        match &self.partida {
            None => {
                let (tantos, abstract_game, max_mus_rounds) =
                    (self.tantos, self.abstract_game, self.max_mus_rounds);
                let partidas = RepartoMusIter::new().map(
                    move |(mano1, mano2, mano3, mano4, probability, dist)| {
                        let mut game =
                            Self::new(tantos, abstract_game, max_mus_rounds).with_hands([
                                Mano::new(mano1),
                                Mano::new(mano2),
                                Mano::new(mano3),
                                Mano::new(mano4),
                            ]);
                        game.set_card_source(CardSource::Iterable(dist));
                        (game, probability)
                    },
                );
                Either::Left(partidas)
            }
            Some(_) => {
                let game = self.clone();
                let descartes = game.partida.as_ref().unwrap().descartadas().unwrap();
                let partidas = match descartes.len() {
                    1 => Either::Left(Either::Left(Self::iter_descartes::<1>(game))),
                    2 => Either::Left(Either::Right(Self::iter_descartes::<2>(game))),
                    3 => Either::Right(Either::Left(Self::iter_descartes::<3>(game))),
                    4 => Either::Right(Either::Right(Self::iter_descartes::<4>(game))),
                    _ => unreachable!(),
                };
                Either::Right(partidas)
            }
        }
    }

    fn current_node(&self) -> NodeType {
        match &self.partida {
            None => NodeType::Chance,
            Some(partida) => match partida.fase() {
                None => NodeType::Terminal,
                Some(FasePartida::DescartePendiente) => NodeType::Chance,
                Some(FasePartida::Mus | FasePartida::Descartes | FasePartida::Envites(_)) => {
                    match partida.turno() {
                        Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                            NodeType::Player(player_id as usize, self.actions().len())
                        }
                        None => NodeType::Terminal,
                    }
                }
            },
        }
    }

    fn act(&self, action_id: usize) -> Self {
        let mut new_game = self.clone();
        let action = new_game.actions()[action_id];
        new_game.act_with_action(action);
        new_game
    }

    fn node_key(&self) -> u64 {
        let mut key = self.info_set_builder.public_history;
        if matches!(self.current_node(), NodeType::Chance) {
            key |= 1 << 63;
        }
        key
    }
}

#[derive(Debug, Clone)]
pub struct MusGameTwoHands {
    tantos: [u8; 2],
    cards: Option<CardSource>,
    partida: Option<PartidaMus<CuatroJugadores>>,
    mus_rounds: u8,
    max_mus_rounds: u8,
    abstract_game: bool,
    info_set_builder: MusInfoSetBuilder,
    utility_table: Option<Rc<[[f64; 40]; 40]>>,
}

impl MusGameTwoHands {
    pub fn new(tantos: [u8; 2], abstract_game: bool, max_mus_rounds: u8) -> Self {
        assert!(
            max_mus_rounds <= MAX_RONDAS_MUS,
            "El máximo de rondas de mus es {MAX_RONDAS_MUS}, se han pedido {max_mus_rounds}."
        );
        Self {
            partida: None,
            cards: None,
            tantos,
            mus_rounds: 0,
            max_mus_rounds,
            abstract_game,
            info_set_builder: MusInfoSetBuilder::new(abstract_game),
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
        self.update_hands(Lance::Grande);
        self.info_set_builder.begin_mus();
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
            // El mus se salta sin pasar por `act`, así que hay que abrir aquí la secuencia de
            // apuestas del primer lance igual que haría la transición Mus -> Envites.
            self.info_set_builder.begin_lance(&lance);
            self.update_hands(lance);
        }
    }

    fn update_hands(&mut self, lance: Lance) {
        let manos = self
            .partida
            .as_ref()
            .expect("La partida debe estar repartida.")
            .manos();
        for (player_idx, mano) in manos.iter().enumerate() {
            self.info_set_builder.set_hand(player_idx, mano, &lance);
        }
    }

    fn set_card_source(&mut self, cartas: CardSource) {
        self.cards = Some(cartas);
    }

    pub fn with_utility_table(self, utility_table: Rc<[[f64; 40]; 40]>) -> Self {
        Self {
            utility_table: Some(utility_table),
            ..self
        }
    }

    fn iter_descartes<const N: usize>(game: Self) -> impl Iterator<Item = (Self, f64)> {
        let Some(CardSource::Iterable(estado_baraja)) = game.cards else {
            panic!("iter_descartes expects an iterable CardSource");
        };
        let iter = RepartoDescarteMusIter::<N>::new(estado_baraja);
        iter.map(move |(nuevas, probability, dist)| {
            let mut new_game = game.clone();
            new_game
                .partida
                .as_mut()
                .expect("Game must exist in descartes phase")
                .descartar_con_nuevas(&nuevas)
                .expect("Game must be expecting a discard but it doesn't");
            new_game.set_card_source(CardSource::Iterable(dist));
            new_game.update_hands(Lance::Grande);
            new_game.enforce_max_mus_rounds();
            (new_game, probability)
        })
    }

    pub fn actions(&self) -> ArrayVec<Accion, 6> {
        let partida = self.partida.as_ref().unwrap();
        debug_assert!(
            !matches!(partida.fase(), Some(FasePartida::Mus))
                || self.mus_rounds < self.max_mus_rounds,
            "Nodo de jugador en la fase de mus sin rondas disponibles:              falta un enforce_max_mus_rounds tras resolver el descarte."
        );
        actions(partida)
    }

    pub fn act_with_action(&mut self, action: Accion) {
        let (player_id, phase, new_phase) = {
            let partida = self
                .partida
                .as_mut()
                .expect("partida must be initialized before calling act");
            let phase = partida.fase();
            let turno = partida.turno().expect("some player must be active");
            let player_id = turno.player_id() as usize;
            let _ = partida.actuar(action);
            if matches!(turno, Turno::Pareja(0 | 1)) && matches!(phase, Some(FasePartida::Envites(_))) {
                partida
                    .actuar(action)
                    .expect("segunda mano de la pareja debe aceptar la misma acción");
            }
            let new_phase = partida.fase();
            (player_id, phase, new_phase)
        };
        let is_first_partner = player_id == 0 || player_id == 1;
        match phase {
            Some(FasePartida::Mus) => {
                if is_first_partner && action != Accion::NoMus {
                    self.info_set_builder.set_hidden_action(Some(action));
                } else {
                    self.info_set_builder.step_mus(action);
                    self.info_set_builder.set_hidden_action(None);
                }
            }
            Some(FasePartida::Envites(_)) => {
                self.info_set_builder.step_lance(action);
            }
            _ => {}
        }
        match (phase, new_phase) {
            (Some(FasePartida::Mus), Some(FasePartida::Descartes)) => {
                self.info_set_builder.begin_descartes();
                self.mus_rounds += 1;
            }
            (Some(FasePartida::Descartes), Some(FasePartida::Mus)) => {
                self.info_set_builder.begin_mus();
            }
            (Some(FasePartida::Descartes), Some(FasePartida::DescartePendiente)) => {
                let descartes = self.partida.as_ref().unwrap().descartadas().unwrap();
                self.info_set_builder.set_descartes(player_id, &descartes);
            }
            (Some(FasePartida::Mus), Some(FasePartida::Envites(lance))) => {
                self.info_set_builder.begin_lance(&lance);
                self.update_hands(lance);
            }
            (
                Some(FasePartida::Envites(lance_previo)),
                Some(FasePartida::Envites(lance_siguiente)),
            ) if lance_previo != lance_siguiente => {
                self.info_set_builder.begin_lance(&lance_siguiente);
                self.update_hands(lance_siguiente);
                let manos = self
                    .partida
                    .as_ref()
                    .expect("partida must be initialized if phase is envites")
                    .manos();
                self.info_set_builder.set_jugada(&lance_siguiente, manos);
            }
            _ => {}
        }
    }
}

impl Game for MusGameTwoHands {
    type InfoSet = MusInfoSet;
    const N_PLAYERS: usize = 2;

    fn utility(&self, player: usize) -> f64 {
        let tantos = self.partida.as_ref().unwrap().tantos();
        utility(player, &tantos, self.utility_table.as_deref())
    }

    fn info_set(&self, player: usize) -> Self::InfoSet {
        // Cada jugador ve las dos manos de su pareja (puestos `player` y `player + 2`), que se
        // empaquetan en la mitad baja y alta de la parte privada del conjunto de información.
        self.info_set_builder.to_mus_infoset_two_hands(player)
    }

    fn chance_sample(&self) -> Self {
        let mut new_game = self.clone();
        match &mut new_game.partida {
            None => {
                let mut baraja = Baraja::baraja_mus();
                let manos = baraja.repartir_manos();
                new_game.set_hands(manos);
                new_game.set_card_source(CardSource::Baraja(baraja));
            }
            Some(p) => {
                if let Some(CardSource::Baraja(baraja)) = &mut new_game.cards {
                    let descartes = p.descartadas().unwrap();
                    let nuevas = baraja.descartar(descartes.into_iter());
                    let _ = p.descartar_con_nuevas(&nuevas);
                    // Las cartas nuevas cambian la mano de quien descartó.
                    new_game.update_hands(Lance::Grande);
                }
            }
        }
        new_game.enforce_max_mus_rounds();

        new_game
    }

    fn chance_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        match &self.partida {
            None => {
                let (tantos, abstract_game, max_mus_rounds) =
                    (self.tantos, self.abstract_game, self.max_mus_rounds);
                let partidas = RepartoMusIter::new().map(
                    move |(mano1, mano2, mano3, mano4, probability, dist)| {
                        let mut game =
                            Self::new(tantos, abstract_game, max_mus_rounds).with_hands([
                                Mano::new(mano1),
                                Mano::new(mano2),
                                Mano::new(mano3),
                                Mano::new(mano4),
                            ]);
                        game.set_card_source(CardSource::Iterable(dist));
                        (game, probability)
                    },
                );
                Either::Left(partidas)
            }
            Some(_) => {
                let game = self.clone();
                let descartes = game.partida.as_ref().unwrap().descartadas().unwrap();
                let games = match descartes.len() {
                    1 => Either::Left(Either::Left(Self::iter_descartes::<1>(game))),
                    2 => Either::Left(Either::Right(Self::iter_descartes::<2>(game))),
                    3 => Either::Right(Either::Left(Self::iter_descartes::<3>(game))),
                    4 => Either::Right(Either::Right(Self::iter_descartes::<4>(game))),
                    _ => unreachable!(),
                };

                Either::Right(games)
            }
        }
    }

    fn current_node(&self) -> NodeType {
        match &self.partida {
            None => NodeType::Chance,
            Some(partida) => match partida.fase() {
                None => NodeType::Terminal,
                Some(FasePartida::DescartePendiente) => NodeType::Chance,
                Some(FasePartida::Mus | FasePartida::Descartes | FasePartida::Envites(_)) => {
                    match partida.turno() {
                        Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                            NodeType::Player(player_id as usize % 2, self.actions().len())
                        }
                        None => NodeType::Terminal,
                    }
                }
            },
        }
    }

    fn act(&self, action_id: usize) -> Self {
        let mut new_game = self.clone();
        let action = new_game.actions()[action_id];
        new_game.act_with_action(action);
        new_game
    }

    fn node_key(&self) -> u64 {
        let mut key = self.info_set_builder.public_history;
        if matches!(self.current_node(), NodeType::Chance) {
            key |= 1 << 63;
        }
        key
    }
}

#[derive(Debug, Clone)]
pub struct MusGameTwoPlayers {
    tantos: [u8; 2],
    cards: Option<CardSource>,
    partida: Option<PartidaMus<DosJugadores>>,
    mus_rounds: u8,
    max_mus_rounds: u8,
    abstract_game: bool,
    info_set_builder: MusInfoSetBuilder,
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
            mus_rounds: 0,
            max_mus_rounds,
            abstract_game,
            info_set_builder: MusInfoSetBuilder::new(abstract_game),
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
        new_game.init_partida_mus(manos);
        new_game
    }

    fn init_partida_mus(&mut self, manos: [Mano; 2]) {
        self.partida = Some(PartidaMus::<DosJugadores>::new(manos, self.tantos));
        self.update_hands(Lance::Grande);
        self.info_set_builder.begin_mus();
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
            // El mus se salta sin pasar por `act`, así que hay que abrir aquí la secuencia de
            // apuestas del primer lance igual que haría la transición Mus -> Envites.
            self.info_set_builder.begin_lance(&lance);
            self.update_hands(lance);
        }
    }

    fn update_hands(&mut self, lance: Lance) {
        let manos = self
            .partida
            .as_ref()
            .expect("La partida debe estar repartida.")
            .manos();
        for (player_idx, mano) in manos.iter().enumerate() {
            self.info_set_builder.set_hand(player_idx, mano, &lance);
        }
    }

    fn update_hand(&mut self, player_id: usize, lance: Lance) {
        let manos = self
            .partida
            .as_ref()
            .expect("La partida debe estar repartida.")
            .manos();
        self.info_set_builder
            .set_hand(player_id, &manos[player_id], &lance);
    }

    fn set_card_source(&mut self, cartas: CardSource) {
        self.cards = Some(cartas);
    }

    pub fn mus_game(&self) -> Option<&PartidaMus<DosJugadores>> {
        self.partida.as_ref()
    }

    fn iter_descartes<const N: usize>(game: Self) -> impl Iterator<Item = (Self, f64)> {
        let Some(CardSource::Iterable(estado_baraja)) = game.cards else {
            panic!("iter_descartes expects an iterable CardSource");
        };
        let iter = RepartoDescarteMusIter::<N>::new(estado_baraja);
        iter.map(move |(nuevas, probability, dist)| {
            let mut new_game = game.clone();
            new_game
                .partida
                .as_mut()
                .expect("Game must exist in descartes phase")
                .descartar_con_nuevas(&nuevas)
                .expect("Game must be expecting a discard but it doesn't");
            new_game.set_card_source(CardSource::Iterable(dist));
            // Las cartas nuevas cambian la mano de quien descartó.
            new_game.update_hands(Lance::Grande);
            new_game.enforce_max_mus_rounds();
            (new_game, probability)
        })
    }

    pub fn actions(&self) -> ArrayVec<Accion, 6> {
        let partida = self.partida.as_ref().unwrap();
        debug_assert!(
            !matches!(partida.fase(), Some(FasePartida::Mus))
                || self.mus_rounds < self.max_mus_rounds,
            "Mus phase reached and no available rounds: missing call to enforce_max_mus_rounds."
        );
        actions(partida)
    }

    pub fn act_with_action(&mut self, action: Accion) {
        let (turno, phase, new_phase) = {
            let partida = self
                .partida
                .as_mut()
                .expect("partida must be initialized before calling act_with_action");
            let phase = partida.fase();
            let turno = partida
                .turno()
                .expect("some player must be active in descartes phase")
                .player_id() as usize;
            let _ = partida.actuar(action);
            let new_phase = partida.fase();
            (turno, phase, new_phase)
        };
        match phase {
            Some(FasePartida::Mus) => {
                self.info_set_builder.step_mus(action);
            }
            Some(FasePartida::Envites(_)) => {
                self.info_set_builder.step_lance(action);
            }
            _ => {}
        }
        match (phase, new_phase) {
            (Some(FasePartida::Mus), Some(FasePartida::Descartes)) => {
                self.info_set_builder.begin_descartes();
                self.mus_rounds += 1;
            }
            (Some(FasePartida::Descartes), Some(FasePartida::Mus)) => {
                self.info_set_builder.begin_mus();
            }
            (Some(FasePartida::Descartes), Some(FasePartida::DescartePendiente)) => {
                let descartes = self.partida.as_ref().unwrap().descartadas().unwrap();
                self.info_set_builder.set_descartes(turno, &descartes);
            }
            (Some(FasePartida::Mus), Some(FasePartida::Envites(lance))) => {
                self.info_set_builder.begin_lance(&lance);
                self.update_hands(lance)
            }
            (
                Some(FasePartida::Envites(lance_previo)),
                Some(FasePartida::Envites(lance_siguiente)),
            ) if lance_previo != lance_siguiente => {
                self.info_set_builder.begin_lance(&lance_siguiente);
                self.update_hands(lance_siguiente);
                let manos = self
                    .partida
                    .as_ref()
                    .expect("partida must be initialized if phase is envites")
                    .manos();
                self.info_set_builder.set_jugada(&lance_siguiente, manos);
            }
            _ => {}
        }
    }
}

impl Game for MusGameTwoPlayers {
    type InfoSet = MusInfoSet;
    const N_PLAYERS: usize = 2;

    fn utility(&self, player: usize) -> f64 {
        let tantos = self.partida.as_ref().unwrap().tantos();
        utility(player, &tantos, self.utility_table.as_deref())
    }

    fn info_set(&self, player: usize) -> Self::InfoSet {
        self.info_set_builder.to_mus_infoset(player)
    }

    fn chance_sample(&self) -> Self {
        let mut new_game = self.clone();
        match &mut new_game.partida {
            None => {
                let mut baraja = Baraja::baraja_mus();
                let manos = baraja.repartir_manos();
                new_game.init_partida_mus(manos);
                new_game.set_card_source(CardSource::Baraja(baraja));
            }
            Some(p) => {
                if let Some(CardSource::Baraja(baraja)) = &mut new_game.cards {
                    let descartes = p.descartadas().unwrap();
                    let nuevas = baraja.descartar(descartes.into_iter());
                    let _ = p.descartar_con_nuevas(&nuevas);
                    // Las cartas nuevas cambian la mano de quien descartó.
                    new_game.update_hands(Lance::Grande);
                }
            }
        }
        new_game.enforce_max_mus_rounds();

        new_game
    }

    fn chance_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        match &self.partida {
            None => {
                let (tantos, abstract_game, max_mus_rounds) =
                    (self.tantos, self.abstract_game, self.max_mus_rounds);
                let games = RepartoMusDosJugadoresIter::new().map(
                    move |(mano1, mano2, probability, dist)| {
                        let mut game = Self::new(tantos, abstract_game, max_mus_rounds)
                            .with_hands([Mano::new(mano1), Mano::new(mano2)]);
                        game.set_card_source(CardSource::Iterable(dist));
                        (game, probability)
                    },
                );
                Either::Left(games)
            }
            Some(_) => {
                let new_game = self.clone();
                let descartes = new_game.partida.as_ref().unwrap().descartadas().unwrap();
                let games = match descartes.len() {
                    1 => Either::Left(Either::Left(Self::iter_descartes::<1>(new_game))),
                    2 => Either::Left(Either::Right(Self::iter_descartes::<2>(new_game))),
                    3 => Either::Right(Either::Left(Self::iter_descartes::<3>(new_game))),
                    4 => Either::Right(Either::Right(Self::iter_descartes::<4>(new_game))),
                    _ => unreachable!(),
                };
                Either::Right(games)
            }
        }
    }

    fn current_node(&self) -> NodeType {
        match &self.partida {
            None => NodeType::Chance,
            Some(partida) => match partida.fase() {
                None => NodeType::Terminal,
                Some(FasePartida::DescartePendiente) => NodeType::Chance,
                Some(FasePartida::Mus | FasePartida::Descartes | FasePartida::Envites(_)) => {
                    match partida.turno() {
                        Some(Turno::Jugador(player_id)) | Some(Turno::Pareja(player_id)) => {
                            NodeType::Player(player_id as usize % 2, self.actions().len())
                        }
                        None => NodeType::Terminal,
                    }
                }
            },
        }
    }

    fn act(&self, action_id: usize) -> Self {
        let mut new_game = self.clone();
        let action = new_game.actions()[action_id];
        new_game.act_with_action(action);
        new_game
    }

    fn node_key(&self) -> u64 {
        let mut key = self.info_set_builder.public_history;
        if matches!(self.current_node(), NodeType::Chance) {
            key |= 1 << 63;
        }
        key
    }
}

fn actions<T: ModalidadMus>(partida: &PartidaMus<T>) -> ArrayVec<Accion, 6> {
    match partida.fase() {
        Some(FasePartida::Mus) => [Accion::Mus, Accion::NoMus].into_iter().collect(),
        Some(FasePartida::Descartes) => {
            let turno = partida
                .turno()
                .expect("Some player must be active to call actions()")
                .player_id() as usize;
            let mano = &partida.manos().as_ref()[turno];
            let mut descartes = [false; 4];
            for (idx, carta) in mano.iter().enumerate() {
                descartes[idx] = *carta != Carta::Rey;
            }
            if descartes == [false; 4] {
                descartes[0] = true;
            }
            [Accion::Descartar(descartes)].into_iter().collect()
        }
        Some(FasePartida::Envites(_)) => {
            let fase_envites = partida.fase_envites().unwrap();
            let ultimo_envite: Apuesta = fase_envites.ultima_apuesta();
            let apuesta_maxima = fase_envites.apuesta_maxima();
            let mut actions = actions_envite(ultimo_envite, apuesta_maxima);
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
        Some(FasePartida::DescartePendiente) => ArrayVec::new(),
        None => todo!(),
    }
}

/// Índice canónico de cada acción, fijo e independiente de qué acciones estén disponibles en cada
/// momento. Es el hijo con el que se indexan `mus_sequence` y `lance_sequence`, de modo que una
/// misma acción (p. ej. órdago) siempre recorre el mismo hijo del árbol, sin que el marcador o la
/// configuración de manos desplacen los índices y provoquen colisiones. Mus y envites usan árboles
/// distintos, así que sus códigos pueden solaparse.
fn canonical_envite_action(action: Accion) -> usize {
    match action {
        Accion::NoMus => 0,
        Accion::Mus => 1,
        Accion::Paso => 0,
        Accion::Quiero => 1,
        Accion::Envido(2) => 2,
        Accion::Envido(5) => 3,
        Accion::Envido(10) => 4,
        Accion::Ordago => 5,
        other => unreachable!("acción inesperada en el árbol de apuestas: {other:?}"),
    }
}

fn actions_envite(ultimo_envite: Apuesta, apuesta_maxima: u8) -> ArrayVec<Accion, 6> {
    match ultimo_envite {
        Apuesta::Tantos(tantos) if tantos == apuesta_maxima => {
            [Accion::Paso, Accion::Quiero, Accion::Ordago]
                .into_iter()
                .collect()
        }
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

#[derive(Debug, Clone)]
enum CardSource {
    Baraja(Baraja),
    Iterable([(Carta, u8); 8]),
}

pub type MusInfoSet = (u64, u64);

#[derive(Debug, Clone)]
struct MusInfoSetBuilder {
    public_history: u64,
    private_history: [u64; 4],

    tables: Arc<MusInfoSetTables>,
    current_lance: Option<u8>,
    current_node: u32,
}

impl MusInfoSetBuilder {
    fn to_mus_infoset(&self, player_id: usize) -> MusInfoSet {
        (self.public_history, self.private_history[player_id])
    }

    fn to_mus_infoset_two_hands(&self, player_id: usize) -> MusInfoSet {
        (
            self.public_history,
            self.private_history[player_id] | (self.private_history[player_id + 2] << 32),
        )
    }

    fn set_hand(&mut self, player_id: usize, mano: &Mano, lance: &Lance) {
        let value = self.tables.rank_hand(mano, lance);
        Self::put(
            &mut self.private_history[player_id],
            value,
            self.tables.mano.offset,
            self.tables.mano.width,
        );
    }

    fn set_descartes(&mut self, player_id: usize, descartes: &[Carta]) {
        let code_offset = match descartes.len() {
            1 => 1,
            2 => 9,
            3 => 45,
            4 => 165,
            _ => 0,
        };

        Self::put(
            &mut self.private_history[player_id],
            code_offset + self.tables.rank_complete_hand(descartes),
            self.tables.descartes.offset,
            self.tables.descartes.width,
        );
        self.step_descartes(descartes.len());
    }

    fn begin_mus(&mut self) {
        self.current_node = 0;
    }

    fn step_mus(&mut self, action: Accion) {
        self.current_node = self.tables.mus_sequence.step(self.current_node, action);
        Self::put(
            &mut self.public_history,
            self.current_node as u64,
            self.tables.history_mus.offset,
            self.tables.history_mus.width,
        )
    }

    fn begin_descartes(&mut self) {
        self.current_node = 1;
    }

    fn step_descartes(&mut self, num_descartes: usize) {
        self.current_node = self.current_node * 4 + (num_descartes - 1) as u32;
        Self::put(
            &mut self.public_history,
            self.current_node as u64,
            self.tables.history_descartes.offset,
            self.tables.history_descartes.width,
        )
    }

    fn begin_lance(&mut self, lance: &Lance) {
        self.current_lance = Some(match lance {
            Lance::Grande => 0,
            Lance::Chica => 1,
            Lance::Pares => 2,
            Lance::Punto => 3,
            Lance::Juego => 3,
        });
        self.current_node = 0;
    }

    fn step_lance(&mut self, action: Accion) {
        let lance_idx =
            self.current_lance
                .expect("begin_lance should be called before step_lance") as usize;
        self.current_node = self.tables.lance_sequence.step(self.current_node, action);
        Self::put(
            &mut self.public_history,
            self.current_node as u64,
            self.tables.history_lance[lance_idx].offset,
            self.tables.history_lance[lance_idx].width,
        )
    }

    fn set_hidden_action(&mut self, action: Option<Accion>) {
        let code = action.map_or(0, |a| 1 + canonical_envite_action(a) as u64);
        Self::put(
            &mut self.public_history,
            code,
            self.tables.hidden_action.offset,
            self.tables.hidden_action.width,
        )
    }

    fn set_jugada(&mut self, lance: &Lance, manos: &[Mano]) {
        match lance {
            Lance::Pares => {
                let jugada = manos
                    .iter()
                    .map(|m| m.pares().is_some() as u8)
                    .fold(0, |acum, v| acum << 1 | v);
                Self::put(
                    &mut self.public_history,
                    jugada as u64,
                    self.tables.jugadas_pares.offset,
                    self.tables.jugadas_pares.width,
                )
            }

            Lance::Juego => {
                let jugada = manos
                    .iter()
                    .map(|m| m.juego().is_some() as u8)
                    .fold(0, |acum, v| acum << 1 | v);
                Self::put(
                    &mut self.public_history,
                    jugada as u64,
                    self.tables.jugadas_juego.offset,
                    self.tables.jugadas_juego.width,
                )
            }
            _ => {}
        }
    }

    fn put(dest: &mut u64, value: u64, offset: u32, width: u32) {
        debug_assert!(value < (1 << width), "field overflow");
        let mask = (1 << width) - 1;
        *dest &= !(mask << offset);
        *dest |= value << offset;
    }

    fn new(abstract_game: bool) -> Self {
        Self {
            public_history: 0,
            private_history: [0; 4],
            tables: Arc::new(MusInfoSetTables::new(abstract_game)),
            current_lance: None,
            current_node: 0,
        }
    }
}

#[derive(Debug)]
struct BitField {
    width: u32,
    offset: u32,
}

#[derive(Debug)]
pub(crate) struct MusInfoSetTables {
    mano: BitField,
    descartes: BitField,
    jugadas_pares: BitField,
    jugadas_juego: BitField,
    history_mus: BitField,
    history_descartes: BitField,
    history_lance: [BitField; 4],
    hidden_action: BitField,

    combinations: [[u64; 5]; 12],
    mus_sequence: BettingSequence,
    lance_sequence: BettingSequence,
    abstract_hands: Option<[HashMap<AbstractJugada, u32>; 5]>,
}

impl MusInfoSetTables {
    pub(crate) fn new(abstract_game: bool) -> Self {
        let combinations = Self::combinations_table();
        let mus_sequence = BettingSequence::from_sequences(&[vec![0], vec![1, 0], vec![1, 1]]);
        let lance_sequence = BettingSequence::from_sequences(&Self::sequences_lance());
        let abstract_hands = abstract_game.then(Self::abstract_hands_table);

        // private part
        let mut offset = 0;
        let mano = BitField { width: 9, offset };
        offset += mano.width;
        let descartes = BitField { width: 9, offset };

        // public part
        let mut offset = 0;
        let jugadas_pares = BitField { width: 4, offset };
        offset += jugadas_pares.width;
        let jugadas_juego = BitField { width: 4, offset };
        offset += jugadas_juego.width;
        let history_mus = BitField {
            width: u32::BITS - (mus_sequence.num_nodes() as u32).leading_zeros(),
            offset,
        };
        offset += history_mus.width;
        let history_descartes = BitField { width: 9, offset };
        offset += history_descartes.width;
        let width_lance = u32::BITS - (lance_sequence.num_nodes() as u32).leading_zeros();
        let history_lance = std::array::from_fn(|_idx| {
            let lance = BitField {
                width: width_lance,
                offset,
            };
            offset += width_lance;
            lance
        });
        let hidden_action = BitField { width: 3, offset };
        Self {
            combinations,
            mus_sequence,
            lance_sequence,
            abstract_hands,
            mano,
            descartes,
            jugadas_pares,
            jugadas_juego,
            history_mus,
            history_descartes,
            history_lance,
            hidden_action,
        }
    }

    pub(crate) fn combinations_table() -> [[u64; 5]; 12] {
        let mut combinations = [[1u64, 0, 0, 0, 0]; 12];
        (1..12usize).for_each(|n| {
            (1..5usize).for_each(|k| {
                combinations[n][k] = num_integer::binomial(n as u64, k as u64);
            });
        });
        combinations
    }

    pub(crate) fn rank_hand(&self, mano: &Mano, lance: &Lance) -> u64 {
        match &self.abstract_hands {
            Some(maps) => {
                let idx = Self::lance_abstract_index(lance);
                AbstractJugada::to_abstract(mano, lance)
                    .and_then(|jugada| maps[idx].get(&jugada).copied())
                    .unwrap_or(0) as u64
            }
            None => self.rank_complete_hand(mano.cartas()),
        }
    }

    fn rank_complete_hand(&self, mano: &[Carta]) -> u64 {
        let mut rank = 0;
        for (i, c) in mano.iter().rev().enumerate() {
            rank += self.combinations[c.valor_mus() as usize + i][i + 1];
        }
        rank
    }

    fn lance_abstract_index(lance: &Lance) -> usize {
        match lance {
            Lance::Grande => 0,
            Lance::Chica => 1,
            Lance::Pares => 2,
            Lance::Juego => 3,
            Lance::Punto => 4,
        }
    }

    fn sequences_lance() -> Vec<Vec<u8>> {
        let mut out = vec![];

        let estado_lance = EstadoLance::<DosJugadores>::new(
            &Lance::Grande,
            &[
                Mano::try_from("RRRR").unwrap(),
                Mano::try_from("R111").unwrap(),
            ],
            40,
        );
        fn a(estado_lance: EstadoLance<DosJugadores>, path: &mut Vec<u8>, out: &mut Vec<Vec<u8>>) {
            let actions =
                actions_envite(estado_lance.ultima_apuesta(), estado_lance.apuesta_maxima());
            for action in actions {
                path.push(canonical_envite_action(action) as u8);
                let mut new_estado_lance = estado_lance.clone();
                match new_estado_lance.actuar(action).unwrap() {
                    Some(_) => a(new_estado_lance, path, out),
                    None => out.push(path.clone()),
                }
                path.pop();
            }
        }
        a(estado_lance, &mut Vec::new(), &mut out);

        out
    }

    pub(crate) fn abstract_hands_table() -> [HashMap<AbstractJugada, u32>; 5] {
        let lances = [
            Lance::Grande,
            Lance::Chica,
            Lance::Pares,
            Lance::Juego,
            Lance::Punto,
        ];
        let mut jugadas: [BTreeSet<AbstractJugada>; 5] = Default::default();
        for cartas in CartaIter::new(&Carta::CARTAS_MUS, 4) {
            let mano = Mano::new(cartas.try_into().expect("CartaIter yields four cards"));
            for (i, lance) in lances.iter().enumerate() {
                if let Some(jugada) = AbstractJugada::to_abstract(&mano, lance) {
                    jugadas[i].insert(jugada);
                }
            }
        }
        core::array::from_fn(|i| {
            jugadas[i]
                .iter()
                .enumerate()
                .map(|(idx, jugada)| (*jugada, idx as u32 + 1))
                .collect()
        })
    }
}

#[derive(Debug)]
struct BettingSequence {
    nodes: Vec<[isize; 6]>,
}

impl BettingSequence {
    fn from_sequences(sequences: &[Vec<u8>]) -> Self {
        let sequences = sequences.to_vec();

        let mut nodes: Vec<[isize; 6]> = vec![[-1; 6]];

        for sequence in &sequences {
            let mut node = 0;
            for action in sequence {
                let mut child = nodes[node as usize][*action as usize];
                if child < 0 {
                    nodes.push([-1; 6]);
                    child = (nodes.len() - 1) as isize;
                    nodes[node as usize][*action as usize] = child;
                }
                node = child;
            }
        }

        Self { nodes }
    }

    fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    fn step(&self, node: u32, action: Accion) -> u32 {
        self.nodes[node as usize][canonical_envite_action(action)] as u32
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;

    use super::*;

    fn cuatro_manos() -> [Mano; 4] {
        [
            Mano::from_str("RRR5").unwrap(),
            Mano::from_str("RCC1").unwrap(),
            Mano::from_str("RRR4").unwrap(),
            Mano::from_str("RCC7").unwrap(),
        ]
    }

    fn dos_manos() -> [Mano; 2] {
        [
            Mano::from_str("RRR5").unwrap(),
            Mano::from_str("RCC1").unwrap(),
        ]
    }

    fn walk<G: Game<InfoSet = MusInfoSet>>(
        game: &G,
        infosets: &mut HashMap<MusInfoSet, usize>,
        budget: &mut usize,
    ) {
        if *budget == 0 {
            return;
        }
        match game.current_node() {
            NodeType::Terminal => {}
            NodeType::Chance => {
                if let Some((child, _p)) = game.chance_iter().next() {
                    walk(&child, infosets, budget);
                }
            }
            NodeType::Player(player, n_actions) => {
                *budget -= 1;
                let info = game.info_set(player);
                let prev = infosets.entry(info).or_insert(n_actions);
                assert_eq!(
                    *prev, n_actions,
                    "conjunto de información {info:?} visto con distinto número de acciones"
                );
                for action_id in 0..n_actions {
                    walk(&game.act(action_id), infosets, budget);
                }
            }
        }
    }

    fn play_one_line<G: Game<InfoSet = MusInfoSet>>(mut game: G) {
        for _ in 0..1_000 {
            match game.current_node() {
                NodeType::Terminal => return,
                NodeType::Chance => {
                    let next = game.chance_iter().next().expect("nodo de azar sin hijos").0;
                    game = next;
                }
                NodeType::Player(player, n_actions) => {
                    let _ = game.info_set(player);
                    game = game.act(n_actions - 1);
                }
            }
        }
        panic!("la partida no terminó en 1000 pasos");
    }

    #[test]
    fn mus_game_full_tree_no_panic() {
        let game = MusGame::new([0, 0], false, 0).with_hands(cuatro_manos());
        // Al reparto cada uno de los cuatro jugadores ve una mano distinta.
        let infos: Vec<_> = (0..4).map(|p| game.info_set(p)).collect();
        for i in 0..4 {
            for j in (i + 1)..4 {
                assert_ne!(infos[i], infos[j], "jugadores {i} y {j} comparten conjunto");
            }
        }
        let mut infosets = HashMap::new();
        walk(&game, &mut infosets, &mut 300_000);
        assert!(!infosets.is_empty());
    }

    #[test]
    fn mus_game_two_hands_full_tree_no_panic() {
        let game = MusGameTwoHands::new([0, 0], false, 0).with_hands(cuatro_manos());
        // Cada jugador ve las dos manos de su pareja: los dos conjuntos son distintos.
        assert_ne!(game.info_set(0), game.info_set(1));
        let mut infosets = HashMap::new();
        walk(&game, &mut infosets, &mut 300_000);
        assert!(!infosets.is_empty());
    }

    #[test]
    fn mus_game_two_players_full_tree_no_panic() {
        let game = MusGameTwoPlayers::new([0, 0], false, 0).with_hands(dos_manos());
        assert_ne!(game.info_set(0), game.info_set(1));
        let mut infosets = HashMap::new();
        walk(&game, &mut infosets, &mut 300_000);
        assert!(!infosets.is_empty());
    }

    #[test]
    fn full_game_with_mus_round_terminates() {
        play_one_line(MusGame::new([0, 0], false, 1));
        play_one_line(MusGameTwoHands::new([0, 0], false, 1));
        play_one_line(MusGameTwoPlayers::new([0, 0], false, 1));
    }

    #[test]
    fn two_players_actions() {
        let manos = dos_manos();
        let mut game = MusGameTwoPlayers::new([35, 35], false, 1).with_hands(manos.clone());
        game.act_with_action(Accion::NoMus);
        game.act_with_action(Accion::Envido(2));
        game.act_with_action(Accion::Envido(2));
        assert_eq!(
            game.actions().to_vec(),
            vec![Accion::Paso, Accion::Quiero, Accion::Ordago]
        );
        let mut game = MusGameTwoPlayers::new([37, 37], false, 1).with_hands(manos.clone());
        game.act_with_action(Accion::NoMus);
        assert_eq!(
            game.actions().to_vec(),
            vec![Accion::Paso, Accion::Envido(2), Accion::Ordago]
        );
        game.act_with_action(Accion::Envido(2));
        assert_eq!(
            game.actions().to_vec(),
            vec![Accion::Paso, Accion::Quiero, Accion::Ordago]
        );
    }

    #[test]
    fn mus_game_hidden_first_partner() {
        let mut game = MusGame::new([38, 37], false, 0).with_hands(cuatro_manos());
        assert!(matches!(game.current_node(), NodeType::Player(0, _)));
        game.act_with_action(Accion::Ordago);
        assert!(matches!(game.current_node(), NodeType::Player(2, _)));
        assert_eq!(game.actions().to_vec(), vec![Accion::Ordago]);
    }

    #[test]
    fn hidden_action_visible_to_partner() {
        let base = MusGame::new([0, 0], false, 0).with_hands(cuatro_manos());

        let mut tras_paso = base.clone();
        tras_paso.act_with_action(Accion::Paso);
        let mut tras_ordago = base.clone();
        tras_ordago.act_with_action(Accion::Ordago);

        assert!(matches!(tras_paso.current_node(), NodeType::Player(2, _)));
        assert!(matches!(tras_ordago.current_node(), NodeType::Player(2, _)));
        assert_ne!(tras_paso.info_set(2), tras_ordago.info_set(2));
    }

    #[test]
    fn abstract_game_merges_equivalent_hands() {
        let mano_a = Mano::from_str("S655").unwrap();
        let mano_b = Mano::from_str("S544").unwrap();
        let opp = Mano::from_str("RRR1").unwrap();
        let abstracto_a =
            MusGameTwoPlayers::new([0, 0], true, 0).with_hands([mano_a.clone(), opp.clone()]);
        let abstracto_b =
            MusGameTwoPlayers::new([0, 0], true, 0).with_hands([mano_b.clone(), opp.clone()]);
        assert_eq!(abstracto_a.info_set(0), abstracto_b.info_set(0));

        let exacto_a = MusGameTwoPlayers::new([0, 0], false, 0).with_hands([mano_a, opp.clone()]);
        let exacto_b = MusGameTwoPlayers::new([0, 0], false, 0).with_hands([mano_b, opp]);
        assert_ne!(exacto_a.info_set(0), exacto_b.info_set(0));
    }

    #[test]
    fn abstract_game_full_tree_no_panic() {
        let game = MusGame::new([0, 0], true, 0).with_hands(cuatro_manos());
        let mut infosets = HashMap::new();
        walk(&game, &mut infosets, &mut 300_000);
        assert!(!infosets.is_empty());
    }
}

#[cfg(test)]
mod fix_tests {
    use super::*;
    use std::collections::HashMap;

    fn walk_collision(game: &MusGame, seen: &mut HashMap<MusInfoSet, usize>, budget: &mut usize) {
        if *budget == 0 {
            return;
        }
        match game.current_node() {
            NodeType::Terminal => {}
            NodeType::Chance => {
                let child = game.chance_sample();
                walk_collision(&child, seen, budget);
            }
            NodeType::Player(player, n) => {
                *budget -= 1;
                let info = game.info_set(player);
                let prev = *seen.entry(info).or_insert(n);
                assert_eq!(
                    prev, n,
                    "colisión en el conjunto {info:?}: {prev} vs {n} acciones"
                );
                for a in 0..n {
                    walk_collision(&game.act(a), seen, budget);
                }
            }
        }
    }

    #[test]
    fn no_infoset_collision_across_deals() {
        for tantos in [[0, 0], [39, 39]] {
            let mut seen = HashMap::new();
            for _ in 0..400 {
                walk_collision(&MusGame::new(tantos, false, 1), &mut seen, &mut 1_500);
            }
        }
    }
}
