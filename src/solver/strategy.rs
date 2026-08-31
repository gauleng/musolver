use std::{
    cmp::Ordering,
    collections::HashMap,
    fmt::Display,
    fs::{self, File},
    path::Path,
    sync::Arc,
};

use arrayvec::ArrayVec;
use memmap2::Mmap;
use walkdir::WalkDir;

use crate::{
    Cfr, Game, NodeType,
    mus::{
        Accion, Baraja, Carta, DistribucionCartaIter, DistribucionDobleCartaIter, FasePartida,
        Lance, Mano, Turno, probabilidad_dos_manos, probabilidad_mano,
    },
    solver::{GenericMus, MusGame, MusGameTwoHands, MusGameTwoPlayers, MusInfoSet},
};

use super::{SolverError, TrainerConfig};

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Serialize,
    rkyv::Deserialize,
    rkyv::Archive,
    Clone,
    Copy,
)]
pub enum GameType {
    LanceGame(Lance),
    LanceGameTwoHands(Lance),
    MusGame,
    MusGameTwoHands,
    MusGameTwoPlayers,
}

impl GameType {
    /// Número de manos que se reparten. No coincide necesariamente con el número de jugadores
    /// que deciden: [`GameType::MusGameTwoHands`] reparte cuatro manos y las agrupa por parejas.
    pub fn num_hands(&self) -> usize {
        match self {
            GameType::MusGame | GameType::MusGameTwoHands => 4,
            GameType::MusGameTwoPlayers => 2,
            GameType::LanceGame(_) | GameType::LanceGameTwoHands(_) => todo!(),
        }
    }
}

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Serialize,
    rkyv::Deserialize,
    rkyv::Archive,
    Clone,
)]
pub struct GameConfig {
    pub game_type: GameType,
    pub abstract_game: bool,
    pub max_mus_rounds: u8,
}

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Serialize,
    rkyv::Deserialize,
    rkyv::Archive,
    Clone,
)]
pub struct StrategyConfig {
    pub trainer_config: TrainerConfig,
    pub game_config: GameConfig,
}

#[derive(Debug)]
pub struct StrategyReader {
    file: Mmap,
    strategy_config: StrategyConfig,
}

/// Configuración de una mano: si tiene pares y si tiene juego. Selecciona la mano de ejemplo que
/// la representa en el cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandConfig {
    pub pares: bool,
    pub juego: bool,
}

impl HandConfig {
    /// Configuración de una mano concreta.
    pub fn of(mano: &Mano) -> Self {
        Self {
            pares: mano.pares().is_some(),
            juego: mano.juego().is_some(),
        }
    }

    /// Indica si la mano tiene los mismos pares y juego que esta configuración. Solo las manos
    /// compatibles dejan intactos el orden de turnos y los lances que se juegan.
    pub fn matches(&self, mano: &Mano) -> bool {
        *self == Self::of(mano)
    }
}

/// Recorrido por el árbol de una estrategia entrenada.
///
/// Cada mano se representa por la mano de ejemplo de su [`HandConfig`], así que el estado del
/// cursor es función pura de `(tantos, hand_configs, moves)`: cualquier cambio en la
/// configuración se aplica reconstruyendo la partida desde el reparto.
///
/// `moves` es la línea que el usuario ha pedido y `history` la parte que se puede jugar de
/// verdad: `history[k + 1]` es `history[k]` tras aplicar `moves[k]`. Normalmente
/// `history.len() == moves.len() + 1`, pero cambiar los tantos o una configuración puede acortar
/// la secuencia de lances y dejar movimientos sin realizar al final. Se conservan: si el cambio se
/// deshace, la línea vuelve entera. Solo [`Cursor::act`] los descarta, al abrir una rama nueva.
///
/// Invariante: `history.len() <= moves.len() + 1` y `position < history.len()`.
pub struct Cursor {
    reader: Arc<StrategyReader>,
    history: Vec<Box<dyn GenericMus>>,
    moves: Vec<CursorMove>,
    position: usize,
    tantos: [u8; 2],
    /// Longitud igual a `game_type.num_hands()`: no hay entradas que el juego ignore.
    hand_configs: ArrayVec<HandConfig, 4>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandKind {
    OneHand(Mano),
    TwoHands(Mano, Mano),
}

pub struct HandStrategy {
    hand: HandKind,
    actions: Vec<Accion>,
    strategy: Vec<f64>,
    reach_probability: f64,
}

impl HandStrategy {
    pub fn hand(&self) -> &HandKind {
        &self.hand
    }

    /// Acciones legales en el nodo, alineadas con [`HandStrategy::strategy`].
    pub fn actions(&self) -> &[Accion] {
        &self.actions
    }

    /// Probabilidad de cada acción, en el mismo orden que [`HandStrategy::actions`].
    pub fn strategy(&self) -> &[f64] {
        &self.strategy
    }

    /// Peso de esta mano en el nodo: su probabilidad a priori por la de que el jugador juegue la
    /// línea recorrida con ella. Sin normalizar entre las manos devueltas.
    pub fn reach_probability(&self) -> f64 {
        self.reach_probability
    }
}

pub enum CursorNode {
    Play(Vec<Accion>),
    Discard,
    Terminal,
}

#[derive(Debug, Clone)]
pub enum CursorMove {
    Play(Accion),
    Discard(DiscardAction),
}

impl Display for CursorMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CursorMove::Play(accion) => write!(f, "{accion}"),
            CursorMove::Discard(DiscardAction::Count(num_descartes)) => {
                write!(f, "descarte de {num_descartes}")
            }
            CursorMove::Discard(DiscardAction::Cards(cartas)) => {
                let cartas: String = cartas.iter().map(char::from).collect();
                write!(f, "descarte de {cartas}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum DiscardAction {
    Count(usize),
    Cards(Vec<Carta>),
}

impl DiscardAction {
    /// Única fuente del número de descartes: nunca puede discrepar de las cartas indicadas.
    pub fn len(&self) -> usize {
        match self {
            DiscardAction::Count(num_descartes) => *num_descartes,
            DiscardAction::Cards(cartas) => cartas.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Cursor {
    fn new(reader: Arc<StrategyReader>) -> Self {
        let num_hands = reader.strategy_config.game_config.game_type.num_hands();
        let hand_configs = (0..num_hands)
            .map(|_| HandConfig {
                pares: true,
                juego: true,
            })
            .collect();
        let mut cursor = Cursor {
            history: Vec::new(),
            reader,
            tantos: [0; 2],
            hand_configs,
            position: 0,
            moves: Vec::new(),
        };
        cursor.history = cursor
            .replay(&[])
            .expect("el reparto inicial no aplica ningún movimiento");
        cursor
    }

    pub fn set_tantos(&mut self, tantos: [u8; 2]) {
        self.tantos = tantos;
        self.retain_valid_line();
    }

    pub fn game_type(&self) -> GameType {
        self.reader.strategy_config.game_config.game_type
    }

    pub fn num_hands(&self) -> usize {
        self.game_type().num_hands()
    }

    pub fn hand_configs(&self) -> &[HandConfig] {
        &self.hand_configs
    }

    pub fn set_hand_config(&mut self, hand: usize, config: HandConfig) -> Result<(), SolverError> {
        let num_hands = self.num_hands();
        if hand >= num_hands {
            return Err(SolverError::InvalidHandIndex(hand, num_hands));
        }
        self.hand_configs[hand] = config;
        // Las jugadas solo entran en el historial público al abrirse su lance, así que cambiar una
        // mitad aún no declarada nunca invalida la línea recorrida.
        self.retain_valid_line();
        Ok(())
    }

    /// Cambia la configuración de todas las manos de una vez.
    ///
    /// Preferible a llamar a [`Cursor::set_hand_config`] varias veces: cada llamada recorta el
    /// registro de movimientos por su cuenta, y una configuración intermedia puede tener menos
    /// lances que la inicial y la final, con lo que se perdería más recorrido del necesario.
    pub fn set_hand_configs(&mut self, configs: &[HandConfig]) -> Result<(), SolverError> {
        let num_hands = self.num_hands();
        if configs.len() != num_hands {
            return Err(SolverError::InvalidHandIndex(configs.len(), num_hands));
        }
        self.hand_configs = configs.iter().copied().collect();
        self.retain_valid_line();
        Ok(())
    }

    /// Mitades de la configuración ya fijadas por la información pública del nodo actual. El resto
    /// son hipótesis: cambiarlas no altera la estrategia aquí, solo los lances que vienen después.
    ///
    /// Las jugadas solo entran en el historial público al abrirse su lance, así que antes de eso
    /// sus bits valen cero sea cual sea la mano.
    pub fn declared(&self) -> HandConfig {
        match self.history[self.position].phase() {
            Some(FasePartida::Envites(Lance::Pares)) => HandConfig {
                pares: true,
                juego: false,
            },
            Some(FasePartida::Envites(Lance::Juego | Lance::Punto)) => HandConfig {
                pares: true,
                juego: true,
            },
            _ => HandConfig {
                pares: false,
                juego: false,
            },
        }
    }

    /// Indica si se puede preguntar por esta mano en el nodo actual.
    ///
    /// Solo tienen que coincidir las jugadas ya declaradas ([`Cursor::declared`]). En mus,
    /// descartes, grande y chica no hay ninguna: los bits de pares y juego del historial público
    /// siguen a cero y todos los jugadores participan en el lance, así que la configuración no
    /// llega al conjunto de información y valen todas las manos.
    ///
    /// A partir de pares sí: cambiar una jugada ya declarada cambiaría los lances que se juegan y
    /// quién participa en ellos, y la partida reproducida ya no sería la que tiene delante el
    /// cursor.
    pub fn accepts_hand(&self, hand: usize, mano: &Mano) -> bool {
        let declared = self.declared();
        let config = self.hand_configs[hand];
        (!declared.pares || config.pares == mano.pares().is_some())
            && (!declared.juego || config.juego == mano.juego().is_some())
    }

    pub fn act(&mut self, action: CursorMove) -> Result<(), SolverError> {
        match (self.cursor_node(), &action) {
            (CursorNode::Play(_), CursorMove::Play(_)) => {}
            (CursorNode::Discard, CursorMove::Discard(discard)) => {
                let num_descartes = discard.len();
                if !(1..=4).contains(&num_descartes) {
                    return Err(SolverError::InvalidDiscardsNumber(num_descartes));
                }
            }
            _ => return Err(SolverError::InvalidCursorMove(action)),
        }
        // Un descarte de cartas concretas cambia la mano repartida, así que invalida el prefijo
        // del historial: se reconstruye la partida entera desde el reparto. Nada se modifica
        // hasta que la reproducción tiene éxito. Actuar abre una rama nueva, así que aquí sí se
        // descarta lo que hubiera después de la posición actual.
        let mut moves = self.moves[..self.position].to_vec();
        moves.push(action);
        let history = self.replay(&moves)?;

        self.position = history.len() - 1;
        self.history = history;
        self.moves = moves;
        Ok(())
    }

    /// Reconstruye la partida desde el reparto aplicando `moves`, con las manos de ejemplo de la
    /// configuración actual.
    fn replay(&self, moves: &[CursorMove]) -> Result<Vec<Box<dyn GenericMus>>, SolverError> {
        self.replay_with(moves, &[])
    }

    /// Reconstruye la partida sustituyendo la mano de los jugadores indicados en `overrides`.
    /// Devuelve el estado inicial seguido del resultante de cada movimiento.
    ///
    /// Sustituir la mano en la partida, en lugar de escribirla en el conjunto de información, es
    /// lo que mantiene alineados `actions()` y el conjunto de información: ambos se derivan del
    /// mismo estado. Escribir solo el conjunto de información funcionaría en los envites, donde
    /// las acciones no dependen de la mano, pero no en la fase de descartes, donde
    /// `actions_descarte` devuelve un número de máscaras distinto según las cartas repetidas.
    ///
    /// La mano sustituida debe ser compatible con la [`HandConfig`] del jugador: el orden de
    /// turnos y los lances que se juegan dependen de la mano solo a través de pares y juego.
    fn replay_with(
        &self,
        moves: &[CursorMove],
        overrides: &[Option<Mano>],
    ) -> Result<Vec<Box<dyn GenericMus>>, SolverError> {
        let manos = self.dealt_hands(moves, overrides);
        let mut game = Self::init_game(
            self.tantos,
            &self.reader.strategy_config.game_config,
            &manos,
        );
        let mut history = Vec::with_capacity(moves.len() + 1);
        history.push(game.clone());

        let mut num_descartes = 0;
        for movimiento in moves {
            // Cambiar la configuración o los tantos puede cambiar los lances que se juegan, y
            // dejar movimientos que ya no valen o que sobran. Hay que detectarlo antes de actuar:
            // `act_with_action` da por hecho que hay jugador de turno y revienta si no lo hay.
            match (Self::node_of(&*game), movimiento) {
                (CursorNode::Play(actions), CursorMove::Play(accion))
                    if actions.contains(accion) =>
                {
                    game.act_with_action(*accion)?
                }
                (CursorNode::Discard, CursorMove::Discard(discard)) => {
                    // La fase de descartes recorre a los jugadores en orden 0..N-1, cada uno
                    // exactamente una vez, así que el descarte j-ésimo es del jugador j.
                    let hand = num_descartes;
                    debug_assert_eq!(
                        game.turn().map(|turno| turno.player_id() as usize),
                        Some(hand),
                        "el registro de descartes no sigue el orden de turnos"
                    );
                    let after = self.target_hand(hand, overrides);
                    Self::apply_discard(&mut game, discard, &after)?;
                    num_descartes += 1;
                }
                _ => return Err(SolverError::InvalidCursorMove(movimiento.clone())),
            }
            history.push(game.clone());
        }
        Ok(history)
    }

    /// Reproduce el prefijo más largo de `moves` que sigue siendo válido, junto con su longitud.
    fn replay_prefix(&self, moves: &[CursorMove]) -> (Vec<Box<dyn GenericMus>>, usize) {
        for len in (0..moves.len()).rev() {
            if let Ok(history) = self.replay(&moves[..=len]) {
                return (history, len + 1);
            }
        }
        (
            self.replay(&[])
                .expect("el reparto sin movimientos siempre se reproduce"),
            0,
        )
    }

    /// Reconstruye la partida conservando la parte del recorrido que sigue siendo jugable.
    ///
    /// Cambiar los tantos o una configuración puede cambiar los lances que se juegan y dejar la
    /// cola del registro sin realizar. No se borra: deshacer el cambio devuelve la línea entera.
    /// Esto importa cuando la configuración se toca de una jugada en una, porque un estado
    /// intermedio puede tener menos lances que el inicial y el final.
    fn retain_valid_line(&mut self) {
        let moves = std::mem::take(&mut self.moves);
        let (history, _) = self.replay_prefix(&moves);
        self.moves = moves;
        self.position = self.position.min(history.len() - 1);
        self.history = history;
    }

    /// Mano con la que se queda un jugador tras sus descartes: la sustituida si se indica y, si no,
    /// la de ejemplo de su configuración.
    fn target_hand(&self, hand: usize, overrides: &[Option<Mano>]) -> Mano {
        overrides
            .get(hand)
            .and_then(|mano| mano.clone())
            .unwrap_or_else(|| Self::example_hand(self.hand_configs[hand]))
    }

    /// Manos del reparto: la mano final de cada jugador ([`Cursor::target_hand`]), salvo que vaya a
    /// descartar cartas concretas, en cuyo caso recibe una mano que las contiene y que, tras el
    /// descarte, vuelve a ser la final.
    ///
    /// La sustitución tiene que entrar aquí y no solo en el `after` de la reproducción: si no, un
    /// descarte de cartas concretas construiría su mano previa a partir de la de ejemplo y el
    /// jugador acabaría con esa en lugar de con la sustituida.
    ///
    /// Supone un único turno de descartes (`max_mus_rounds <= 1`). Con más rondas habría que
    /// encadenar hacia atrás: cada reparto entregaría la mano previa al siguiente descarte y solo
    /// el último restauraría la mano final.
    fn dealt_hands(&self, moves: &[CursorMove], overrides: &[Option<Mano>]) -> ArrayVec<Mano, 4> {
        let mut manos: ArrayVec<Mano, 4> = (0..self.hand_configs.len())
            .map(|hand| self.target_hand(hand, overrides))
            .collect();
        let descartes = moves
            .iter()
            .filter_map(|movimiento| match movimiento {
                CursorMove::Discard(discard) => Some(discard),
                CursorMove::Play(_) => None,
            })
            .take(manos.len());
        for (hand, discard) in descartes.enumerate() {
            if let DiscardAction::Cards(cartas) = discard {
                manos[hand] = Self::pre_discard_hand(&manos[hand], cartas);
            }
        }
        manos
    }

    /// Mano previa al descarte: las cartas que se van a descartar más las `4 - n` mejores de la
    /// mano resultante, que son las que el jugador conserva.
    ///
    /// Siempre es una mano legal: cuatro cartas no pueden superar la multiplicidad de ningún valor
    /// en la baraja de mus.
    fn pre_discard_hand(after: &Mano, descartes: &[Carta]) -> Mano {
        let mut cartas: ArrayVec<Carta, 4> = after.cartas()[..4 - descartes.len()]
            .iter()
            .copied()
            .collect();
        cartas.extend(descartes.iter().copied());
        Mano::from_arrayvec(cartas)
    }

    /// Descarta y reparte en un solo paso, dejando al jugador de turno con la mano `after`.
    ///
    /// Las cartas repartidas son las que le faltan a la mano para llegar a `after`, así que un
    /// [`DiscardAction::Count`] devuelve exactamente las descartadas y la mano no cambia.
    fn apply_discard(
        game: &mut Box<dyn GenericMus>,
        discard: &DiscardAction,
        after: &Mano,
    ) -> Result<(), SolverError> {
        let hand = game
            .turn()
            .expect("algún jugador debe estar activo en la fase de descartes")
            .player_id() as usize;
        let mano = game.hands()[hand].clone();
        let mask = Self::discard_mask_for(&mano, discard)?;

        let conservadas: ArrayVec<Carta, 4> = mano
            .cartas()
            .iter()
            .enumerate()
            .filter_map(|(idx, carta)| (!mask[idx]).then_some(*carta))
            .collect();
        let nuevas = Self::missing_cards(after, &conservadas);

        game.act_with_action(Accion::Descartar(mask))?;
        game.deal_new_cards(&nuevas)
    }

    /// Máscara con la que se ejecuta un descarte sobre `mano`.
    fn discard_mask_for(mano: &Mano, discard: &DiscardAction) -> Result<[bool; 4], SolverError> {
        match discard {
            // Máscara prefijo: siempre está entre las que genera `actions_descarte`, sea cual sea
            // el patrón de cartas repetidas, porque marcar el bit k+1 exige tener marcado el k.
            DiscardAction::Count(num_descartes) => {
                Ok(std::array::from_fn(|idx| idx < *num_descartes))
            }
            DiscardAction::Cards(cartas) => Self::discard_mask(mano, cartas),
        }
    }

    /// Marca en `mano` las posiciones de `descartes`. Para cada carta se elige la posición libre
    /// más a la izquierda con ese valor, que es el representante que conserva `actions_descarte`:
    /// dentro de una racha de cartas iguales, marcar la posición k+1 exige tener marcada la k.
    fn discard_mask(mano: &Mano, descartes: &[Carta]) -> Result<[bool; 4], SolverError> {
        let mut mask = [false; 4];
        for carta in descartes {
            let idx = mano
                .cartas()
                .iter()
                .enumerate()
                .find(|(idx, c)| !mask[*idx] && *c == carta)
                .map(|(idx, _)| idx)
                .ok_or_else(|| {
                    SolverError::InvalidCursorMove(CursorMove::Discard(DiscardAction::Cards(
                        descartes.to_vec(),
                    )))
                })?;
            mask[idx] = true;
        }
        Ok(mask)
    }

    /// Cartas que faltan en `conservadas` para completar `objetivo`, como diferencia de
    /// multiconjuntos.
    fn missing_cards(objetivo: &Mano, conservadas: &[Carta]) -> ArrayVec<Carta, 4> {
        let mut restantes: ArrayVec<Carta, 4> = conservadas.iter().copied().collect();
        objetivo
            .cartas()
            .iter()
            .filter(|carta| match restantes.iter().position(|c| c == *carta) {
                Some(idx) => {
                    restantes.remove(idx);
                    false
                }
                None => true,
            })
            .copied()
            .collect()
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Estrategia del jugador de turno suponiendo que tiene las manos indicadas. Devuelve `None`
    /// en un nodo terminal o si el conjunto de información no está en la estrategia entrenada.
    ///
    /// También devuelve `None` si la mano contradice una jugada ya declarada en el nodo: ver
    /// [`Cursor::accepts_hand`]. En grande y chica no hay ninguna declarada, así que se puede
    /// preguntar por cualquier mano.
    pub fn strategy_for_hand(&self, hand: &HandKind) -> Result<Option<HandStrategy>, SolverError> {
        if matches!(self.cursor_node(), CursorNode::Terminal) {
            return Ok(None);
        }
        let player = self.decision_maker_of(
            self.turn()
                .expect("un nodo no terminal tiene jugador de turno")
                .player_id() as usize,
        );
        let two_hands = matches!(
            self.reader.strategy_config.game_config.game_type,
            GameType::MusGameTwoHands
        );

        let mut overrides = self.no_overrides();
        let prior = match (hand, two_hands) {
            (HandKind::OneHand(mano), false) => {
                if !self.accepts_hand(player, mano) {
                    return Ok(None);
                }
                overrides[player] = Some(mano.clone());
                probabilidad_mano(Baraja::FREC_BARAJA_MUS, mano.cartas())
            }
            (HandKind::TwoHands(mano1, mano2), true) => {
                if !self.accepts_hand(player, mano1) || !self.accepts_hand(player + 2, mano2) {
                    return Ok(None);
                }
                overrides[player] = Some(mano1.clone());
                overrides[player + 2] = Some(mano2.clone());
                probabilidad_dos_manos(Baraja::FREC_BARAJA_MUS, mano1.cartas(), mano2.cartas())
            }
            (_, two_hands) => {
                return Err(SolverError::WrongHandCount(if two_hands { 2 } else { 1 }));
            }
        };

        self.hand_strategy(player, hand.clone(), overrides, prior)
    }

    /// Estrategia del jugador de turno para cada mano compatible con su configuración.
    ///
    /// Se enumeran todas las manos que sirven para el nodo actual ([`Cursor::accepts_hand`]): en
    /// mus, descartes, grande y chica no hay jugadas declaradas todavía y valen todas; a partir de
    /// pares se restringen a las que respetan las ya declaradas.
    ///
    /// Se omiten las manos cuyo conjunto de información no está en la estrategia entrenada.
    pub fn strategies(&self) -> Result<Vec<HandStrategy>, SolverError> {
        if matches!(self.cursor_node(), CursorNode::Terminal) {
            return Ok(Vec::new());
        }
        let hand = self
            .turn()
            .expect("un nodo no terminal tiene jugador de turno")
            .player_id() as usize;
        let player = self.decision_maker_of(hand);

        if matches!(
            self.reader.strategy_config.game_config.game_type,
            GameType::MusGameTwoHands
        ) {
            return self.strategies_two_hands(player);
        }

        DistribucionCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS)
            .filter_map(|(cartas, prior)| {
                let mano = Mano::new(cartas);
                self.accepts_hand(player, &mano).then_some((mano, prior))
            })
            .filter_map(|(mano, prior)| {
                let mut overrides = self.no_overrides();
                overrides[player] = Some(mano.clone());
                self.hand_strategy(player, HandKind::OneHand(mano), overrides, prior)
                    .transpose()
            })
            .collect()
    }

    /// Estrategias del jugador en [`GameType::MusGameTwoHands`], donde maneja las manos de los
    /// puestos `player` y `player + 2`.
    ///
    /// Los pares se enumeran con su probabilidad conjunta: las cartas de la primera mano
    /// condicionan las de la segunda, así que no es el producto de dos repartos independientes.
    fn strategies_two_hands(&self, player: usize) -> Result<Vec<HandStrategy>, SolverError> {
        DistribucionDobleCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS)
            .filter_map(|(cartas1, cartas2, prior)| {
                let mano1 = Mano::new(cartas1);
                let mano2 = Mano::new(cartas2);
                (self.accepts_hand(player, &mano1) && self.accepts_hand(player + 2, &mano2))
                    .then_some((mano1, mano2, prior))
            })
            .filter_map(|(mano1, mano2, prior)| {
                let mut overrides = self.no_overrides();
                overrides[player] = Some(mano1.clone());
                overrides[player + 2] = Some(mano2.clone());
                self.hand_strategy(player, HandKind::TwoHands(mano1, mano2), overrides, prior)
                    .transpose()
            })
            .collect()
    }

    fn no_overrides(&self) -> ArrayVec<Option<Mano>, 4> {
        self.hand_configs.iter().map(|_| None).collect()
    }

    /// Jugador que decide sobre la mano indicada. En [`GameType::MusGameTwoHands`] cada jugador
    /// maneja dos manos, así que las de los puestos 2 y 3 las deciden los jugadores 0 y 1.
    fn decision_maker_of(&self, hand: usize) -> usize {
        match self.reader.strategy_config.game_config.game_type {
            GameType::MusGameTwoHands => hand % 2,
            _ => hand,
        }
    }

    /// Estrategia del jugador en el nodo actual suponiendo que tiene las manos indicadas. Devuelve
    /// `None` si el conjunto de información no está en la estrategia entrenada.
    ///
    /// Las manos se sustituyen en la partida y se reproduce la línea entera: las acciones y el
    /// conjunto de información salen así del mismo estado y no pueden desalinearse. Escribirlas
    /// solo en el conjunto de información valdría en los envites, donde las acciones no dependen
    /// de la mano, pero no en la fase de descartes.
    fn hand_strategy(
        &self,
        player: usize,
        hand: HandKind,
        overrides: ArrayVec<Option<Mano>, 4>,
        prior: f64,
    ) -> Result<Option<HandStrategy>, SolverError> {
        // Solo hasta la posición actual: `history[position]` lo producen `moves[..position]`, y
        // la cola puede tener movimientos que ahora mismo no se juegan.
        let history = self.replay_with(&self.moves[..self.position], &overrides)?;

        let game = &history[self.position];
        let actions = game.actions().to_vec();
        let Some(strategy) = self
            .reader
            .strategy(self.tantos, &game.mus_info_set(player))
        else {
            return Ok(None);
        };
        debug_assert_eq!(actions.len(), strategy.len());

        let reach_probability = prior * self.own_reach(player, &history)?;
        Ok(Some(HandStrategy {
            hand,
            actions,
            strategy,
            reach_probability,
        }))
    }

    /// Probabilidad de que el jugador juegue por sí solo la línea recorrida hasta el nodo actual:
    /// producto de las probabilidades de sus propias acciones. No incluye las de los demás
    /// jugadores ni las del azar, así que multiplicada por la probabilidad a priori de la mano da
    /// el peso de esa mano en el nodo.
    ///
    /// Un nodo intermedio que no esté en la estrategia entrenada cuenta como inalcanzable.
    fn own_reach(
        &self,
        player: usize,
        history: &[Box<dyn GenericMus>],
    ) -> Result<f64, SolverError> {
        let mut reach = 1.;
        for (idx, movimiento) in self.moves[..self.position].iter().enumerate() {
            let game = &history[idx];
            let Some(hand) = game.turn().map(|turno| turno.player_id() as usize) else {
                continue;
            };
            if self.decision_maker_of(hand) != player {
                continue;
            }
            // El descarte se aplica sobre la mano que tiene el turno, que en
            // `MusGameTwoHands` no es la del jugador que decide.
            let accion = match movimiento {
                CursorMove::Play(accion) => *accion,
                CursorMove::Discard(discard) => {
                    Accion::Descartar(Self::discard_mask_for(&game.hands()[hand], discard)?)
                }
            };
            let Some(pos) = game.actions().iter().position(|a| *a == accion) else {
                return Ok(0.);
            };
            let Some(strategy) = self
                .reader
                .strategy(self.tantos, &game.mus_info_set(player))
            else {
                return Ok(0.);
            };
            reach *= strategy.get(pos).copied().unwrap_or(0.);
        }
        Ok(reach)
    }

    pub fn strategies_for_kept(&self, kept: &HandKind) -> Result<Vec<HandStrategy>, SolverError> {
        todo!();
    }

    pub fn cursor_node(&self) -> CursorNode {
        Self::node_of(&*self.history[self.position()])
    }

    fn node_of(game: &dyn GenericMus) -> CursorNode {
        match game.phase() {
            Some(phase) => match phase {
                FasePartida::Mus | FasePartida::Envites(_) => {
                    CursorNode::Play(game.actions().to_vec())
                }
                FasePartida::Descartes => CursorNode::Discard,
                FasePartida::DescartePendiente => {
                    unreachable!("apply_discard resuelve decisión y azar en un solo paso")
                }
            },
            None => CursorNode::Terminal,
        }
    }

    pub fn turn(&self) -> Option<Turno> {
        self.history[self.position].turn()
    }

    pub fn phase(&self) -> Option<FasePartida> {
        self.history[self.position].phase()
    }

    pub fn seek(&mut self, new_position: usize) {
        self.position = new_position.min(self.history.len() - 1)
    }

    pub fn go_back(&mut self) {
        self.position = self.position.saturating_sub(1);
    }

    pub fn go_forward(&mut self) {
        self.seek(self.position + 1);
    }

    pub fn position(&self) -> usize {
        self.position
    }

    fn init_game(tantos: [u8; 2], game_config: &GameConfig, manos: &[Mano]) -> Box<dyn GenericMus> {
        debug_assert_eq!(manos.len(), game_config.game_type.num_hands());
        let abstract_game = game_config.abstract_game;
        let max_mus_rounds = game_config.max_mus_rounds;
        match game_config.game_type {
            GameType::LanceGame(_lance) => todo!(),
            GameType::LanceGameTwoHands(_lance) => todo!(),
            GameType::MusGame => {
                let manos: [Mano; 4] = std::array::from_fn(|i| manos[i].clone());
                Box::new(MusGame::new(tantos, abstract_game, max_mus_rounds).with_hands(manos))
            }
            GameType::MusGameTwoHands => {
                let manos: [Mano; 4] = std::array::from_fn(|i| manos[i].clone());
                Box::new(
                    MusGameTwoHands::new(tantos, abstract_game, max_mus_rounds).with_hands(manos),
                )
            }
            GameType::MusGameTwoPlayers => {
                let manos: [Mano; 2] = std::array::from_fn(|i| manos[i].clone());
                Box::new(
                    MusGameTwoPlayers::new(tantos, abstract_game, max_mus_rounds).with_hands(manos),
                )
            }
        }
    }

    fn example_hand(config: HandConfig) -> Mano {
        match (config.pares, config.juego) {
            (false, false) => Mano::new([Carta::Seis, Carta::Cinco, Carta::Cuatro, Carta::As]),
            (true, false) => Mano::new([Carta::As, Carta::As, Carta::As, Carta::As]),
            (false, true) => Mano::new([Carta::Rey, Carta::Caballo, Carta::Sota, Carta::As]),
            (true, true) => Mano::new([Carta::Rey, Carta::Rey, Carta::Rey, Carta::Rey]),
        }
    }
}

impl StrategyReader {
    pub fn from_rkyv(path: impl AsRef<Path>) -> Result<Self, SolverError> {
        let file = File::open(path.as_ref()).map_err(|err| {
            SolverError::InvalidStrategyPath(err, path.as_ref().display().to_string())
        })?;
        let file = unsafe { Mmap::map(&file) }.map_err(|err| {
            SolverError::InvalidStrategyPath(err, path.as_ref().display().to_string())
        })?;
        let strategy = unsafe { rkyv::access_unchecked::<ArchivedStrategy>(&file) };

        let strategy_config =
            rkyv::deserialize::<StrategyConfig, rkyv::rancor::Error>(&strategy.strategy_config)
                .map_err(SolverError::ParseStrategyRkyvError)?;
        Ok(Self {
            file,
            strategy_config,
        })
    }

    pub fn cursor(self: Arc<Self>) -> Cursor {
        Cursor::new(self)
    }

    pub fn strategy_config(&self) -> &StrategyConfig {
        &self.strategy_config
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, SolverError> {
        let path = path.as_ref();

        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rkyv") => Self::from_rkyv(path),
            _ => Err(SolverError::UnsupportedFileFormat(
                path.display().to_string(),
            )),
        }
    }

    pub fn strategy_node(
        &self,
        mano1: &Mano,
        mano2: Option<&Mano>,
        tantos: [u8; 2],
        jugadas: &[(bool, bool)],
        history: &[Accion],
    ) -> Option<(Vec<Accion>, Vec<f64>)> {
        let mut manos: Vec<Mano> = jugadas
            .iter()
            .map(|(pares, juego)| Self::example_hand(*pares, *juego))
            .collect();
        let game_type = self.strategy_config.game_config.game_type;
        let GameStateResult(_, NodeType::Player(player_id, _), _) =
            self.game_state(tantos, jugadas, history)
        else {
            return None;
        };
        match game_type {
            GameType::MusGame => {
                manos[player_id] = mano1.clone();
                self.actions(&manos, tantos, history)
            }
            GameType::MusGameTwoHands => {
                manos[player_id] = mano1.clone();
                manos[player_id + 2] = mano2.unwrap().clone();
                self.actions(&manos, tantos, history)
            }
            GameType::MusGameTwoPlayers => {
                manos[player_id] = mano1.clone();
                self.actions(&manos, tantos, history)
            }
            _ => todo!(),
        }
    }

    pub fn actions(
        &self,
        manos: &[Mano],
        tantos: [u8; 2],
        history: &[Accion],
    ) -> Option<(Vec<Accion>, Vec<f64>)> {
        match self.strategy_config.game_config.game_type {
            GameType::LanceGame(_) => todo!(),
            GameType::LanceGameTwoHands(_) => todo!(),
            GameType::MusGame => {
                let manos = [
                    manos[0].clone(),
                    manos[1].clone(),
                    manos[2].clone(),
                    manos[3].clone(),
                ];
                let mus_game = MusGame::new(
                    tantos,
                    self.strategy_config.game_config.abstract_game,
                    self.strategy_config.game_config.max_mus_rounds,
                )
                .with_hands(manos);
                self.actions_for_game(tantos, mus_game, history)
            }
            GameType::MusGameTwoHands => todo!(),
            GameType::MusGameTwoPlayers => {
                let manos = [manos[0].clone(), manos[1].clone()];
                let mus_game = MusGameTwoPlayers::new(
                    tantos,
                    self.strategy_config.game_config.abstract_game,
                    self.strategy_config.game_config.max_mus_rounds,
                )
                .with_hands(manos);
                self.actions_for_game(tantos, mus_game, history)
            }
        }
    }

    pub fn game_state(
        &self,
        tantos: [u8; 2],
        jugadas: &[(bool, bool)],
        history: &[Accion],
    ) -> GameStateResult {
        let manos: Vec<Mano> = jugadas
            .iter()
            .map(|(pares, juego)| Self::example_hand(*pares, *juego))
            .collect();
        match self.strategy_config.game_config.game_type {
            GameType::LanceGame(_) => todo!(),
            GameType::LanceGameTwoHands(_) => todo!(),
            GameType::MusGame => {
                let mut game = MusGame::new(
                    tantos,
                    self.strategy_config.game_config.abstract_game,
                    self.strategy_config.game_config.max_mus_rounds,
                )
                .with_hands([
                    manos[0].clone(),
                    manos[1].clone(),
                    manos[2].clone(),
                    manos[3].clone(),
                ]);
                history
                    .iter()
                    .for_each(|action| game.act_with_action(*action).unwrap());
                let mus_game = game.mus_game();
                let lance = mus_game.unwrap().fase().unwrap();
                let turno = game.current_node();
                let actions = game.actions();
                GameStateResult(lance, turno, actions.to_vec())
            }
            GameType::MusGameTwoHands => todo!(),
            GameType::MusGameTwoPlayers => {
                let mut game = MusGameTwoPlayers::new(
                    tantos,
                    self.strategy_config.game_config.abstract_game,
                    self.strategy_config.game_config.max_mus_rounds,
                )
                .with_hands([manos[0].clone(), manos[1].clone()]);
                history
                    .iter()
                    .for_each(|action| game.act_with_action(*action).unwrap());
                let mus_game = game.mus_game();
                let lance = mus_game.unwrap().fase().unwrap();
                let turno = game.current_node();
                let actions = game.actions();
                GameStateResult(lance, turno, actions.to_vec())
            }
        }
    }

    fn archived(&self) -> &ArchivedStrategy {
        unsafe { rkyv::access_unchecked::<ArchivedStrategy>(&self.file) }
    }

    fn example_hand(pares: bool, juego: bool) -> Mano {
        match (pares, juego) {
            (false, false) => Mano::new([Carta::Seis, Carta::Cinco, Carta::Cuatro, Carta::As]),
            (true, false) => Mano::new([Carta::As, Carta::As, Carta::As, Carta::As]),
            (false, true) => Mano::new([Carta::Rey, Carta::Caballo, Carta::Sota, Carta::As]),
            (true, true) => Mano::new([Carta::Rey, Carta::Rey, Carta::Rey, Carta::Rey]),
        }
    }

    fn actions_for_game<G: GenericMus + Game<InfoSet = MusInfoSet>>(
        &self,
        tantos: [u8; 2],
        game: G,
        history: &[Accion],
    ) -> Option<(Vec<Accion>, Vec<f64>)> {
        let mut game = game;
        for action in history {
            game.act_with_action(*action);
        }
        let turno = match game.current_node() {
            NodeType::Player(t, _) => t,
            NodeType::Terminal | NodeType::Chance => return None,
        };
        let actions = game.actions().to_vec();
        let info_set = game.info_set(turno);
        let strategy = self.strategy(tantos, &info_set);
        Some(actions).zip(strategy)
    }

    fn strategy(&self, tantos: [u8; 2], info_set: &MusInfoSet) -> Option<Vec<f64>> {
        let archived = &self.archived().nodes[tantos[0] as usize][tantos[1] as usize];
        archived
            .get_with(info_set, |q, k| k == q)
            .map(|bytes| bytes.iter().map(|v| f64::from(*v) / 100.).collect())
    }
    //pub fn best_response_value(
    //    &self,
    //    hand1: &Mano,
    //    hand2: &Mano,
    //    action_node: &ActionNode<usize, Accion>,
    //    history: &[Accion],
    //    player: usize,
    //    opponent_hands: &[(Mano, Mano, f64)],
    //) -> f64 {
    //    match action_node {
    //        ActionNode::Terminal => {
    //            let opponent_dist_total: f64 = opponent_hands.iter().map(|(_, _, p)| p).sum();
    //            let mut expected_payoff = 0.;
    //            for (opponent_hand1, opponent_hand2, probability) in opponent_hands {
    //                let opponent_dist = probability / opponent_dist_total;
    //                let hands = [
    //                    hand1.clone(),
    //                    opponent_hand1.clone(),
    //                    hand2.clone(),
    //                    opponent_hand2.clone(),
    //                ];
    //                let mut lance_game = LanceGame::from_partida_mus(
    //                    &PartidaMus::new_partida_lance(
    //                        self.strategy_config.game_config.lance.unwrap(),
    //                        hands,
    //                        [0, 0],
    //                    )
    //                    .unwrap(),
    //                    false,
    //                );
    //                if let Some(l) = &mut lance_game {
    //                    expected_payoff += opponent_dist * l.utility(player);
    //                }
    //            }
    //            expected_payoff
    //        }
    //        ActionNode::NonTerminal(acting_player, children) => {
    //            let mut new_opponent_hands = opponent_hands.to_owned();
    //            let mut weights = vec![0.; children.len()];
    //            let mut util = vec![0.; children.len()];
    //            let mut max_util = 0.;
    //            for (idx_action, (action, next_node)) in children.iter().enumerate() {
    //                if player != *acting_player {
    //                    for (idx_hands, (opponent_hand1, opponent_hand2, prob)) in
    //                        opponent_hands.iter().enumerate()
    //                    {
    //                        let hands = [
    //                            hand1.clone(),
    //                            opponent_hand1.clone(),
    //                            hand2.clone(),
    //                            opponent_hand2.clone(),
    //                        ];
    //                        let lance_game = LanceGame::from_partida_mus(
    //                            &PartidaMus::new_partida_lance(
    //                                self.strategy_config.game_config.lance.unwrap(),
    //                                hands,
    //                                [0, 0],
    //                            )
    //                            .unwrap(),
    //                            self.strategy_config.game_config.abstract_game,
    //                        )
    //                        .unwrap();
    //                        let info_set_str = lance_game.info_set_str(*acting_player);
    //                        let strategy = self.nodes.get(&info_set_str).unwrap();
    //                        new_opponent_hands[idx_hands].2 = prob * strategy[idx_action];
    //                        weights[idx_action] += new_opponent_hands[idx_hands].2;
    //                    }
    //                }
    //                let mut new_history = history.to_vec();
    //                new_history.push(*action);
    //                util[idx_action] = self.best_response_value(
    //                    hand1,
    //                    hand2,
    //                    next_node,
    //                    &new_history,
    //                    player,
    //                    &new_opponent_hands,
    //                );
    //                if player == *acting_player && util[idx_action] > max_util {
    //                    max_util = util[idx_action];
    //                }
    //            }
    //            if player != *acting_player {
    //                let sum_weights: f64 = weights.iter().sum();
    //                let normalized_weights = weights.iter().map(|w| w / sum_weights);
    //                max_util = zip(util.iter(), normalized_weights)
    //                    .map(|(u, w)| u * w)
    //                    .sum();
    //            }
    //            max_util
    //        }
    //    }
    //}

    pub fn find(path: impl AsRef<Path>) -> Vec<(String, StrategyConfig)> {
        let walker = WalkDir::new(path)
            .sort_by(|a, b| match (a.metadata(), b.metadata()) {
                (Ok(metadata_a), Ok(metadata_b)) => {
                    match (metadata_a.modified(), metadata_b.modified()) {
                        (Ok(modified_a), Ok(modified_b)) => modified_a.cmp(&modified_b),
                        _ => Ordering::Less,
                    }
                }
                _ => Ordering::Less,
            })
            .into_iter();
        let mut result = Vec::new();
        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("json") => {
                    let contents = match fs::read_to_string(path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    #[derive(Debug, serde::Deserialize)]
                    struct MockStrategy {
                        strategy_config: StrategyConfig,
                    }
                    let mock_strategy: MockStrategy = match serde_json::from_str(&contents) {
                        Ok(s) => s,
                        Err(_) => continue,
                    };
                    result.push((path.display().to_string(), mock_strategy.strategy_config));
                }
                Some("rkyv") => {
                    let bytes = match fs::read(path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    let archived =
                        match rkyv::access::<ArchivedStrategy, rkyv::rancor::Error>(&bytes) {
                            Ok(s) => s,
                            Err(_) => continue,
                        };
                    let strategy_config =
                        match rkyv::deserialize::<StrategyConfig, rkyv::rancor::Error>(
                            &archived.strategy_config,
                        ) {
                            Ok(s) => s,
                            Err(_) => continue,
                        };
                    result.push((path.display().to_string(), strategy_config));
                }
                _ => {}
            }
        }
        result
    }
}

#[derive(
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Serialize,
    rkyv::Deserialize,
    rkyv::Archive,
)]

pub struct Strategy {
    pub strategy_config: StrategyConfig,
    pub nodes: Vec<Vec<HashMap<MusInfoSet, Vec<u8>>>>,
}

pub struct GameStateResult(pub FasePartida, pub NodeType, pub Vec<Accion>);

impl Strategy {
    pub fn new<G: Game<InfoSet = MusInfoSet> + Send + Sync>(
        cfr: [[Cfr<G>; 40]; 40],
        trainer_config: &TrainerConfig,
        game_config: &GameConfig,
    ) -> Self {
        let nodes = cfr
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cfr| {
                        cfr.into_iter()
                            .map(|(info_set, node)| {
                                let avg_strategy: Vec<u8> = node
                                    .get_average_strategy()
                                    .into_iter()
                                    .map(|v| (v * 100.).round() as u8)
                                    .collect();
                                (info_set.to_owned(), avg_strategy)
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        Self {
            strategy_config: StrategyConfig {
                trainer_config: trainer_config.clone(),
                game_config: game_config.clone(),
            },
            nodes,
        }
    }

    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), SolverError> {
        let contents = serde_json::to_string(self).map_err(SolverError::ParseStrategyJsonError)?;
        fs::write(path.as_ref(), contents).map_err(|err| {
            SolverError::InvalidStrategyPath(err, path.as_ref().display().to_string())
        })?;
        Ok(())
    }

    pub fn to_rkyv(&self, path: impl AsRef<Path>) -> Result<(), SolverError> {
        let contents = rkyv::to_bytes::<rkyv::rancor::Error>(self)
            .map_err(SolverError::ParseStrategyRkyvError)?;
        fs::write(path.as_ref(), contents).map_err(|err| {
            SolverError::InvalidStrategyPath(err, path.as_ref().display().to_string())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Valores con los que se generan descartes: incluyen los que aparecen en las manos de
    /// ejemplo, para cubrir el caso difícil de rachas de cartas iguales.
    const VALORES: [Carta; 4] = [Carta::As, Carta::Cuatro, Carta::Sota, Carta::Rey];

    /// Todos los multiconjuntos de 1 a 4 cartas sobre [`VALORES`], ordenados como una mano.
    fn descartes_posibles() -> Vec<Vec<Carta>> {
        let base = VALORES.len();
        let mut out: Vec<Vec<Carta>> = Vec::new();
        for num_descartes in 1..=4u32 {
            for code in 0..base.pow(num_descartes) {
                let mut cartas: Vec<Carta> = (0..num_descartes)
                    .map(|i| VALORES[(code / base.pow(i)) % base])
                    .collect();
                cartas.sort_unstable_by(|a, b| b.cmp(a));
                if !out.contains(&cartas) {
                    out.push(cartas);
                }
            }
        }
        out
    }

    fn hand_configs() -> Vec<HandConfig> {
        [false, true]
            .into_iter()
            .flat_map(|pares| [false, true].map(|juego| HandConfig { pares, juego }))
            .collect()
    }

    /// Partida de dos jugadores llevada hasta la fase de descartes, con el turno en el jugador 0.
    fn game_in_descartes(manos: [Mano; 2]) -> Box<dyn GenericMus> {
        let mut game: Box<dyn GenericMus> =
            Box::new(MusGameTwoPlayers::new([0, 0], false, 1).with_hands(manos));
        while matches!(game.phase(), Some(FasePartida::Mus)) {
            game.act_with_action(Accion::Mus).unwrap();
        }
        assert_eq!(game.phase(), Some(FasePartida::Descartes));
        assert_eq!(game.turn().map(|turno| turno.player_id()), Some(0));
        game
    }

    #[test]
    fn pre_discard_hand_contains_the_discards() {
        for config in hand_configs() {
            let after = Cursor::example_hand(config);
            for descartes in descartes_posibles() {
                let mano = Cursor::pre_discard_hand(&after, &descartes);
                assert_eq!(mano.cartas().len(), 4);
                // `discard_mask` solo tiene éxito si encuentra una posición libre distinta para
                // cada carta, así que también comprueba las multiplicidades.
                assert!(
                    Cursor::discard_mask(&mano, &descartes).is_ok(),
                    "{mano} no contiene {descartes:?}"
                );
            }
        }
    }

    /// La máscara elegida para unas cartas concretas tiene que ser una de las que genera
    /// `actions_descarte`, o el nodo quedaría fuera del árbol de la estrategia.
    #[test]
    fn discard_mask_is_a_legal_action() {
        for config in hand_configs() {
            let after = Cursor::example_hand(config);
            for descartes in descartes_posibles() {
                let mano = Cursor::pre_discard_hand(&after, &descartes);
                let mask = Cursor::discard_mask(&mano, &descartes).unwrap();
                let game = game_in_descartes([mano.clone(), after.clone()]);
                assert!(
                    game.actions().contains(&Accion::Descartar(mask)),
                    "máscara {mask:?} ilegal para {mano} descartando {descartes:?}"
                );
            }
        }
    }

    /// La máscara prefijo que usa `DiscardAction::Count` es legal para cualquier patrón de cartas
    /// repetidas.
    #[test]
    fn prefix_mask_is_a_legal_action() {
        for config in hand_configs() {
            let mano = Cursor::example_hand(config);
            for num_descartes in 1..=4 {
                let mask: [bool; 4] = std::array::from_fn(|idx| idx < num_descartes);
                let game = game_in_descartes([mano.clone(), mano.clone()]);
                assert!(
                    game.actions().contains(&Accion::Descartar(mask)),
                    "máscara prefijo {mask:?} ilegal para {mano}"
                );
            }
        }
    }

    /// Tras el descarte la mano vuelve a ser la de ejemplo, así que la configuración de pares y
    /// juego del cursor se mantiene exactamente.
    #[test]
    fn cards_discard_restores_the_example_hand() {
        for config in hand_configs() {
            let after = Cursor::example_hand(config);
            for descartes in descartes_posibles() {
                let mano = Cursor::pre_discard_hand(&after, &descartes);
                let mut game = game_in_descartes([mano.clone(), after.clone()]);
                Cursor::apply_discard(&mut game, &DiscardAction::Cards(descartes.clone()), &after)
                    .unwrap();
                assert_eq!(
                    game.hands()[0],
                    after,
                    "descartando {descartes:?} desde {mano}"
                );
            }
        }
    }

    /// Un descarte por número devuelve las mismas cartas: la mano no cambia en absoluto.
    #[test]
    fn count_discard_leaves_the_hand_unchanged() {
        for config in hand_configs() {
            let mano = Cursor::example_hand(config);
            for num_descartes in 1..=4 {
                let mut game = game_in_descartes([mano.clone(), mano.clone()]);
                Cursor::apply_discard(&mut game, &DiscardAction::Count(num_descartes), &mano)
                    .unwrap();
                assert_eq!(game.hands()[0], mano);
            }
        }
    }

    /// `Count(n)` y `Cards(d)` con `d.len() == n` dejan el mismo historial público: el número de
    /// descartes es lo único que entra en él.
    #[test]
    fn count_and_cards_agree_on_public_history() {
        for config in hand_configs() {
            let after = Cursor::example_hand(config);
            for descartes in descartes_posibles() {
                let mano = Cursor::pre_discard_hand(&after, &descartes);

                let mut por_cartas = game_in_descartes([mano.clone(), after.clone()]);
                Cursor::apply_discard(
                    &mut por_cartas,
                    &DiscardAction::Cards(descartes.clone()),
                    &after,
                )
                .unwrap();

                let mut por_numero = game_in_descartes([after.clone(), after.clone()]);
                Cursor::apply_discard(
                    &mut por_numero,
                    &DiscardAction::Count(descartes.len()),
                    &after,
                )
                .unwrap();

                assert_eq!(
                    por_cartas.info_set_builder().public_history(),
                    por_numero.info_set_builder().public_history(),
                    "descartando {descartes:?}"
                );
            }
        }
    }

    #[test]
    fn missing_cards_is_a_multiset_difference() {
        let objetivo = Mano::new([Carta::Rey, Carta::Rey, Carta::Sota, Carta::As]);
        assert_eq!(
            Cursor::missing_cards(&objetivo, &[Carta::Rey, Carta::As]).to_vec(),
            vec![Carta::Rey, Carta::Sota]
        );
        assert_eq!(Cursor::missing_cards(&objetivo, objetivo.cartas()).len(), 0);
        assert_eq!(Cursor::missing_cards(&objetivo, &[]).len(), 4);
    }
}

/// Sufijo distinto en cada llamada, para que dos tests en paralelo no compartan fichero.
#[cfg(test)]
fn unique_id() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod cursor_tests {
    use super::*;
    use crate::CfrMethod;

    /// Escribe una estrategia sin nodos y devuelve un cursor sobre ella. La navegación por el
    /// árbol solo depende de `strategy_config`, así que `nodes` vacío basta.
    fn cursor(game_type: GameType, max_mus_rounds: u8) -> Cursor {
        let strategy = Strategy {
            strategy_config: StrategyConfig {
                trainer_config: TrainerConfig {
                    method: CfrMethod::Cfr,
                    iterations: 0,
                    workers: 1,
                },
                game_config: GameConfig {
                    game_type,
                    abstract_game: false,
                    max_mus_rounds,
                },
            },
            nodes: Vec::new(),
        };
        // Cada fichero tiene que ser único: `from_rkyv` mapea el fichero en memoria, así que
        // reescribir la misma ruta desde otro test en paralelo corrompería un lector vivo.
        let path = std::env::temp_dir().join(format!(
            "musolver-cursor-test-{:?}-{max_mus_rounds}-{}.rkyv",
            game_type,
            unique_id()
        ));
        strategy.to_rkyv(&path).unwrap();
        Arc::new(StrategyReader::from_rkyv(&path).unwrap()).cursor()
    }

    fn example_hands(cursor: &Cursor) -> Vec<Mano> {
        cursor
            .hand_configs()
            .iter()
            .map(|config| Cursor::example_hand(*config))
            .collect()
    }

    /// Lleva el cursor hasta la fase de descartes diciendo mus.
    fn go_to_descartes(cursor: &mut Cursor) {
        while matches!(cursor.phase(), Some(FasePartida::Mus)) {
            cursor.act(CursorMove::Play(Accion::Mus)).unwrap();
        }
        assert_eq!(cursor.phase(), Some(FasePartida::Descartes));
    }

    #[test]
    fn hand_configs_are_sized_by_game_type() {
        assert_eq!(
            cursor(GameType::MusGameTwoPlayers, 1).hand_configs().len(),
            2
        );
        assert_eq!(cursor(GameType::MusGame, 1).hand_configs().len(), 4);
        assert_eq!(cursor(GameType::MusGameTwoHands, 1).hand_configs().len(), 4);
    }

    #[test]
    fn set_hand_config_rejects_hands_the_game_does_not_deal() {
        let mut cursor = cursor(GameType::MusGameTwoPlayers, 1);
        let config = HandConfig {
            pares: false,
            juego: false,
        };
        assert!(cursor.set_hand_config(1, config).is_ok());
        assert!(matches!(
            cursor.set_hand_config(2, config),
            Err(SolverError::InvalidHandIndex(2, 2))
        ));
    }

    /// Tras cualquier descarte las manos vuelven a ser las de ejemplo, así que la configuración de
    /// pares y juego del cursor se mantiene a lo largo de toda la partida.
    #[test]
    fn hands_end_up_matching_the_configuration() {
        let mut cursor = cursor(GameType::MusGameTwoPlayers, 1);
        cursor
            .set_hand_config(
                0,
                HandConfig {
                    pares: false,
                    juego: true,
                },
            )
            .unwrap();
        let esperadas = example_hands(&cursor);

        go_to_descartes(&mut cursor);
        cursor
            .act(CursorMove::Discard(DiscardAction::Cards(vec![
                Carta::Cuatro,
                Carta::Cuatro,
            ])))
            .unwrap();
        cursor
            .act(CursorMove::Discard(DiscardAction::Count(3)))
            .unwrap();

        let final_state = &cursor.history[cursor.position()];
        assert_eq!(final_state.hands(), esperadas.as_slice());
        // El descarte ha terminado: la partida ha pasado a los envites.
        assert!(matches!(cursor.phase(), Some(FasePartida::Envites(_))));
    }

    /// La mano repartida a quien descarta cartas concretas las contiene, y las de los demás son
    /// directamente las de ejemplo.
    #[test]
    fn dealt_hand_contains_the_discarded_cards() {
        let mut cursor = cursor(GameType::MusGameTwoPlayers, 1);
        let esperadas = example_hands(&cursor);
        go_to_descartes(&mut cursor);

        let descartes = vec![Carta::Cuatro, Carta::Cinco];
        cursor
            .act(CursorMove::Discard(DiscardAction::Cards(descartes.clone())))
            .unwrap();

        let repartidas = cursor.history[0].hands();
        for carta in &descartes {
            assert!(
                repartidas[0].cartas().contains(carta),
                "la mano repartida {} no contiene {carta:?}",
                repartidas[0]
            );
        }
        assert_eq!(repartidas[1], esperadas[1]);
    }

    #[test]
    fn replay_is_deterministic() {
        let mut cursor = cursor(GameType::MusGameTwoPlayers, 1);
        go_to_descartes(&mut cursor);
        cursor
            .act(CursorMove::Discard(DiscardAction::Cards(vec![Carta::Rey])))
            .unwrap();
        cursor
            .act(CursorMove::Discard(DiscardAction::Count(2)))
            .unwrap();

        let repetida = cursor.replay(&cursor.moves.clone()).unwrap();
        assert_eq!(repetida.len(), cursor.history.len());
        for (nueva, vieja) in repetida.iter().zip(cursor.history.iter()) {
            assert_eq!(nueva.hands(), vieja.hands());
            assert_eq!(nueva.phase(), vieja.phase());
            assert_eq!(
                nueva.info_set_builder().public_history(),
                vieja.info_set_builder().public_history()
            );
        }
    }

    /// `act` mantiene la invariante `history.len() == moves.len() + 1` y avanza la posición.
    #[test]
    fn act_advances_the_position() {
        let mut cursor = cursor(GameType::MusGameTwoPlayers, 1);
        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.history_len(), 1);

        cursor.act(CursorMove::Play(Accion::Mus)).unwrap();
        assert_eq!(cursor.position(), 1);
        assert_eq!(cursor.history_len(), 2);

        cursor.act(CursorMove::Play(Accion::Mus)).unwrap();
        assert_eq!(cursor.position(), 2);
        assert_eq!(cursor.history_len(), 3);
    }

    /// Actuar tras retroceder abre una rama nueva: descarta la continuación anterior en lugar de
    /// añadirse al final.
    #[test]
    fn act_after_seek_replaces_the_continuation() {
        let mut cursor = cursor(GameType::MusGameTwoPlayers, 1);
        go_to_descartes(&mut cursor);
        let en_descartes = cursor.position();

        cursor
            .act(CursorMove::Discard(DiscardAction::Count(4)))
            .unwrap();
        cursor
            .act(CursorMove::Discard(DiscardAction::Count(4)))
            .unwrap();
        assert_eq!(cursor.history_len(), en_descartes + 3);

        cursor.seek(en_descartes);
        cursor
            .act(CursorMove::Discard(DiscardAction::Count(1)))
            .unwrap();
        assert_eq!(cursor.position(), en_descartes + 1);
        assert_eq!(cursor.history_len(), en_descartes + 2);
        assert_eq!(cursor.moves.len(), en_descartes + 1);
    }

    #[test]
    fn moves_are_rejected_when_they_do_not_match_the_node() {
        let mut cursor = cursor(GameType::MusGameTwoPlayers, 1);
        assert!(matches!(
            cursor.act(CursorMove::Discard(DiscardAction::Count(2))),
            Err(SolverError::InvalidCursorMove(_))
        ));
        go_to_descartes(&mut cursor);
        assert!(matches!(
            cursor.act(CursorMove::Play(Accion::Mus)),
            Err(SolverError::InvalidCursorMove(_))
        ));
        assert!(matches!(
            cursor.act(CursorMove::Discard(DiscardAction::Count(5))),
            Err(SolverError::InvalidDiscardsNumber(5))
        ));
        // Un movimiento rechazado no altera el cursor.
        assert_eq!(cursor.phase(), Some(FasePartida::Descartes));
    }

    /// Antes de abrirse el lance de juego, su mitad de la configuración no está declarada.
    #[test]
    fn declared_grows_with_the_lance() {
        let cursor = cursor(GameType::MusGameTwoPlayers, 0);
        assert!(matches!(cursor.phase(), Some(FasePartida::Envites(_))));
        assert_eq!(
            cursor.declared(),
            HandConfig {
                pares: false,
                juego: false
            }
        );
    }

    #[test]
    fn seek_is_clamped_to_the_history() {
        let mut cursor = cursor(GameType::MusGameTwoPlayers, 1);
        cursor.act(CursorMove::Play(Accion::Mus)).unwrap();
        cursor.seek(99);
        assert_eq!(cursor.position(), cursor.history_len() - 1);
        cursor.go_forward();
        assert_eq!(cursor.position(), cursor.history_len() - 1);
        cursor.go_back();
        assert_eq!(cursor.position(), 0);
        cursor.go_back();
        assert_eq!(cursor.position(), 0);
    }
}

#[cfg(test)]
mod strategies_tests {
    use super::*;
    use crate::CfrMethod;

    /// Estrategia uniforme sobre `n` acciones, en el formato de porcentajes que guarda el fichero.
    fn uniform(n: usize) -> Vec<u8> {
        vec![(100 / n) as u8; n]
    }

    fn write_cursor(
        game_type: GameType,
        max_mus_rounds: u8,
        nodes: HashMap<MusInfoSet, Vec<u8>>,
    ) -> Cursor {
        let strategy = Strategy {
            strategy_config: StrategyConfig {
                trainer_config: TrainerConfig {
                    method: CfrMethod::Cfr,
                    iterations: 0,
                    workers: 1,
                },
                game_config: GameConfig {
                    game_type,
                    abstract_game: false,
                    max_mus_rounds,
                },
            },
            // `StrategyReader::strategy` indexa por tantos, así que la matriz 40x40 debe existir
            // aunque solo se use la casilla [0][0].
            nodes: (0..40)
                .map(|t1| {
                    (0..40)
                        .map(|t2| {
                            if t1 == 0 && t2 == 0 {
                                nodes.clone()
                            } else {
                                HashMap::new()
                            }
                        })
                        .collect()
                })
                .collect(),
        };
        // Ruta única: ver el comentario en `cursor_tests::cursor`.
        let path = std::env::temp_dir().join(format!(
            "musolver-strategies-test-{game_type:?}-{max_mus_rounds}-{}.rkyv",
            unique_id()
        ));
        strategy.to_rkyv(&path).unwrap();
        Arc::new(StrategyReader::from_rkyv(&path).unwrap()).cursor()
    }

    /// Manos por las que se puede preguntar en el nodo actual del cursor.
    fn queryable_hands(cursor: &Cursor, hand: usize) -> Vec<(Mano, f64)> {
        DistribucionCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS)
            .filter_map(|(cartas, prior)| {
                let mano = Mano::new(cartas);
                cursor.accepts_hand(hand, &mano).then_some((mano, prior))
            })
            .collect()
    }

    /// Recorre la línea con cada mano compatible y devuelve una entrada uniforme para cada nodo de
    /// decisión del jugador, con la longitud que corresponde a sus acciones.
    fn tree_entries(
        cursor: &Cursor,
        player: usize,
        moves: &[CursorMove],
    ) -> HashMap<MusInfoSet, Vec<u8>> {
        let mut entries = HashMap::new();
        for (mano, _) in queryable_hands(cursor, player) {
            let mut overrides: ArrayVec<Option<Mano>, 4> =
                cursor.hand_configs.iter().map(|_| None).collect();
            overrides[player] = Some(mano);
            for game in cursor.replay_with(moves, &overrides).unwrap() {
                if game.turn().map(|turno| turno.player_id() as usize) == Some(player) {
                    let num_actions = game.actions().len();
                    if num_actions > 0 {
                        entries.insert(game.mus_info_set(player), uniform(num_actions));
                    }
                }
            }
        }
        entries
    }

    /// Como `tree_entries`, pero para el jugador que maneja las manos `player` y `player + 2`.
    fn tree_entries_two_hands(
        cursor: &Cursor,
        player: usize,
        moves: &[CursorMove],
    ) -> HashMap<MusInfoSet, Vec<u8>> {
        let config1 = cursor.hand_configs[player];
        let config2 = cursor.hand_configs[player + 2];
        let mut entries = HashMap::new();
        for (cartas1, cartas2, _) in
            DistribucionDobleCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS)
        {
            let (mano1, mano2) = (Mano::new(cartas1), Mano::new(cartas2));
            if !config1.matches(&mano1) || !config2.matches(&mano2) {
                continue;
            }
            let mut overrides: ArrayVec<Option<Mano>, 4> =
                cursor.hand_configs.iter().map(|_| None).collect();
            overrides[player] = Some(mano1);
            overrides[player + 2] = Some(mano2);
            for game in cursor.replay_with(moves, &overrides).unwrap() {
                let Some(hand) = game.turn().map(|turno| turno.player_id() as usize) else {
                    continue;
                };
                if cursor.decision_maker_of(hand) != player {
                    continue;
                }
                let num_actions = game.actions().len();
                if num_actions > 0 {
                    entries.insert(game.mus_info_set(player), uniform(num_actions));
                }
            }
        }
        entries
    }

    /// Una mano que no está en la estrategia entrenada se omite en lugar de dar error.
    #[test]
    fn hands_missing_from_the_tree_are_skipped() {
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 1, HashMap::new());
        assert!(cursor.strategies().unwrap().is_empty());
    }

    /// La estrategia devuelta tiene una entrada por acción legal del nodo: es la alineación que
    /// garantiza sustituir la mano en la partida en vez de en el conjunto de información.
    #[test]
    fn strategies_are_aligned_with_the_actions() {
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 1, HashMap::new());
        let entries = tree_entries(&cursor, 0, &[]);
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 1, entries);

        let strategies = cursor.strategies().unwrap();
        let esperadas = queryable_hands(&cursor, 0).len();
        assert_eq!(strategies.len(), esperadas);
        assert!(esperadas > 1);

        for hand_strategy in &strategies {
            assert_eq!(
                hand_strategy.actions().len(),
                hand_strategy.strategy().len()
            );
            let HandKind::OneHand(mano) = hand_strategy.hand() else {
                panic!("se esperaba una sola mano");
            };
            assert!(cursor.accepts_hand(0, mano));
        }
    }

    /// Cada mano tiene que producir su propio conjunto de información: si se leyera el de la mano
    /// de ejemplo, todas las manos compartirían entrada en el árbol.
    #[test]
    fn each_hand_gets_its_own_info_set() {
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        let mut entries = tree_entries(&cursor, 0, &[]);
        // Una probabilidad distinta por conjunto de información: si la sustitución no llegara a la
        // búsqueda, todas las manos devolverían la misma.
        for (idx, valor) in entries.values_mut().enumerate() {
            valor[0] = (idx % 90) as u8;
        }
        assert!(
            entries.len() > 1,
            "las manos comparten conjunto de información"
        );

        let cursor = write_cursor(GameType::MusGameTwoPlayers, 0, entries);
        let strategies = cursor.strategies().unwrap();
        let primera = strategies[0].strategy()[0];
        assert!(
            strategies.iter().any(|s| s.strategy()[0] != primera),
            "todas las manos han devuelto la misma estrategia"
        );
    }

    /// En la raíz el jugador no ha actuado todavía, así que el peso de cada mano es su
    /// probabilidad a priori.
    #[test]
    fn reach_at_the_root_is_the_prior() {
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 1, HashMap::new());
        let entries = tree_entries(&cursor, 0, &[]);
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 1, entries);

        let total: f64 = cursor
            .strategies()
            .unwrap()
            .iter()
            .map(|s| s.reach_probability())
            .sum();
        let prior: f64 = queryable_hands(&cursor, 0)
            .iter()
            .map(|(_, prior)| prior)
            .sum();
        assert!((total - prior).abs() < 1e-9, "{total} != {prior}");
        // En la raíz no hay jugadas declaradas, así que valen todas las manos.
        assert!((total - 1.).abs() < 1e-9, "{total}");
    }

    /// En `MusGameTwoHands` el jugador maneja dos manos, así que se enumeran pares con su
    /// probabilidad conjunta y cada resultado lleva las dos.
    #[test]
    fn two_hands_enumerates_pairs() {
        let cursor = write_cursor(GameType::MusGameTwoHands, 1, HashMap::new());
        let entries = tree_entries_two_hands(&cursor, 0, &[]);
        assert!(!entries.is_empty());
        let cursor = write_cursor(GameType::MusGameTwoHands, 1, entries);

        let strategies = cursor.strategies().unwrap();
        assert!(strategies.len() > 1);
        let mut distintas = 0;
        for hand_strategy in &strategies {
            assert_eq!(
                hand_strategy.actions().len(),
                hand_strategy.strategy().len()
            );
            let HandKind::TwoHands(mano1, mano2) = hand_strategy.hand() else {
                panic!("se esperaban dos manos");
            };
            assert!(cursor.accepts_hand(0, mano1));
            assert!(cursor.accepts_hand(2, mano2));
            if mano1 != mano2 {
                distintas += 1;
            }
            assert!(hand_strategy.reach_probability() > 0.);
        }
        assert!(distintas > 0, "las dos manos son siempre iguales");
    }

    /// Las dos manos tienen que entrar en el conjunto de información. Si solo entrara la primera,
    /// todos los pares que la comparten devolverían la misma estrategia.
    #[test]
    fn two_hands_second_hand_reaches_the_info_set() {
        let cursor = write_cursor(GameType::MusGameTwoHands, 0, HashMap::new());
        let mut entries = tree_entries_two_hands(&cursor, 0, &[]);
        for (idx, valor) in entries.values_mut().enumerate() {
            valor[0] = (idx % 90) as u8;
        }
        let cursor = write_cursor(GameType::MusGameTwoHands, 0, entries);

        // Estrategias agrupadas por la primera mano: dentro de un grupo solo cambia la segunda.
        let mut por_primera: HashMap<Mano, Vec<f64>> = HashMap::new();
        for hand_strategy in cursor.strategies().unwrap() {
            let HandKind::TwoHands(mano1, _) = hand_strategy.hand() else {
                panic!("se esperaban dos manos");
            };
            por_primera
                .entry(mano1.clone())
                .or_default()
                .push(hand_strategy.strategy()[0]);
        }
        let grupos: Vec<_> = por_primera.values().filter(|v| v.len() > 1).collect();
        assert!(!grupos.is_empty(), "no hay pares con la misma primera mano");
        assert!(
            grupos
                .iter()
                .any(|valores| valores.iter().any(|v| *v != valores[0])),
            "la segunda mano no cambia el conjunto de información"
        );
    }

    /// En la fase de descartes el turno recorre las cuatro manos, así que las manos 2 y 3 las
    /// decide el jugador 0 o el 1. El nodo de la mano 2 pertenece al jugador 0.
    #[test]
    fn two_hands_strategies_at_a_discard_node() {
        let mut cursor = write_cursor(GameType::MusGameTwoHands, 1, HashMap::new());
        while matches!(cursor.phase(), Some(FasePartida::Mus)) {
            cursor.act(CursorMove::Play(Accion::Mus)).unwrap();
        }
        // Descartan las manos 0 y 1: el cursor queda en el nodo de la mano 2.
        cursor
            .act(CursorMove::Discard(DiscardAction::Count(1)))
            .unwrap();
        cursor
            .act(CursorMove::Discard(DiscardAction::Count(1)))
            .unwrap();
        assert_eq!(cursor.turn().map(|turno| turno.player_id()), Some(2));
        assert_eq!(cursor.decision_maker_of(2), 0);

        let moves = cursor.moves.clone();
        let entries = tree_entries_two_hands(&cursor, 0, &moves);
        let mut cursor = write_cursor(GameType::MusGameTwoHands, 1, entries);
        for movimiento in moves {
            cursor.act(movimiento).unwrap();
        }

        let strategies = cursor.strategies().unwrap();
        assert!(!strategies.is_empty());
        for hand_strategy in &strategies {
            // Las acciones de un nodo de descartes dependen de la mano que descarta, que aquí es
            // la segunda del jugador: si no se sustituyera, no cuadrarían con la estrategia.
            assert_eq!(
                hand_strategy.actions().len(),
                hand_strategy.strategy().len()
            );
            let HandKind::TwoHands(mano1, mano2) = hand_strategy.hand() else {
                panic!("se esperaban dos manos");
            };
            assert!(cursor.accepts_hand(0, mano1));
            assert!(cursor.accepts_hand(2, mano2));
        }
    }

    /// Preguntar por una mano concreta tiene que dar exactamente lo mismo que buscarla en el
    /// resultado de `strategies()`: si no, mezclar las dos llamadas daría cifras incoherentes.
    #[test]
    fn strategy_for_hand_agrees_with_strategies() {
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        let moves = vec![
            CursorMove::Play(Accion::Paso),
            CursorMove::Play(Accion::Paso),
        ];
        let entries = tree_entries(&cursor, 0, &moves);
        let mut cursor = write_cursor(GameType::MusGameTwoPlayers, 0, entries);
        for movimiento in moves {
            cursor.act(movimiento).unwrap();
        }

        let todas = cursor.strategies().unwrap();
        assert!(todas.len() > 1);
        for esperada in &todas {
            let obtenida = cursor
                .strategy_for_hand(esperada.hand())
                .unwrap()
                .expect("la mano viene de strategies(), tiene que estar en el árbol");
            assert_eq!(obtenida.hand(), esperada.hand());
            assert_eq!(obtenida.actions(), esperada.actions());
            assert_eq!(obtenida.strategy(), esperada.strategy());
            assert!(
                (obtenida.reach_probability() - esperada.reach_probability()).abs() < 1e-12,
                "{} != {}",
                obtenida.reach_probability(),
                esperada.reach_probability()
            );
        }
    }

    #[test]
    fn strategy_for_hand_agrees_with_strategies_two_hands() {
        let cursor = write_cursor(GameType::MusGameTwoHands, 0, HashMap::new());
        let entries = tree_entries_two_hands(&cursor, 0, &[]);
        let cursor = write_cursor(GameType::MusGameTwoHands, 0, entries);

        let todas = cursor.strategies().unwrap();
        assert!(todas.len() > 1);
        for esperada in todas.iter().take(50) {
            let obtenida = cursor.strategy_for_hand(esperada.hand()).unwrap().unwrap();
            assert_eq!(obtenida.strategy(), esperada.strategy());
            assert!((obtenida.reach_probability() - esperada.reach_probability()).abs() < 1e-12);
        }
    }

    /// En grande no hay ninguna jugada declarada, así que se puede preguntar por cualquier mano
    /// aunque contradiga la configuración de pares y juego.
    #[test]
    fn any_hand_is_queryable_before_pares() {
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        assert_eq!(cursor.phase(), Some(FasePartida::Envites(Lance::Grande)));
        assert_eq!(
            cursor.declared(),
            HandConfig {
                pares: false,
                juego: false
            }
        );
        // La configuración por defecto es con pares y con juego.
        let sin_pares_ni_juego = Mano::new([Carta::Seis, Carta::Cinco, Carta::Cuatro, Carta::As]);
        assert!(!cursor.hand_configs()[0].matches(&sin_pares_ni_juego));
        assert!(cursor.accepts_hand(0, &sin_pares_ni_juego));

        // Sin entradas en el árbol devuelve None, pero por no estar entrenada, no por la mano.
        assert!(matches!(
            cursor.strategy_for_hand(&HandKind::OneHand(sin_pares_ni_juego)),
            Ok(None)
        ));
    }

    /// `strategies()` en grande enumera todas las manos, no solo las de la configuración.
    #[test]
    fn strategies_before_pares_covers_every_hand() {
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        let entries = tree_entries(&cursor, 0, &[]);
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 0, entries);

        let strategies = cursor.strategies().unwrap();
        let total = DistribucionCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS).count();
        assert_eq!(strategies.len(), total);

        // Y la masa de probabilidad es la del reparto completo.
        let suma: f64 = strategies.iter().map(|s| s.reach_probability()).sum();
        assert!((suma - 1.).abs() < 1e-9, "{suma}");
    }

    /// Cambiar una jugada aún no declarada en mitad de grande o chica no toca el recorrido: esos
    /// lances no dependen de pares ni de juego.
    #[test]
    fn changing_an_undeclared_jugada_keeps_the_line() {
        let mut cursor = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        cursor.act(CursorMove::Play(Accion::Paso)).unwrap();
        cursor.act(CursorMove::Play(Accion::Paso)).unwrap();
        assert_eq!(cursor.phase(), Some(FasePartida::Envites(Lance::Chica)));

        cursor
            .set_hand_config(
                0,
                HandConfig {
                    pares: true,
                    juego: false,
                },
            )
            .unwrap();

        assert_eq!(cursor.moves.len(), 2);
        assert_eq!(cursor.position(), 2);
        assert_eq!(cursor.phase(), Some(FasePartida::Envites(Lance::Chica)));
        // La mano repartida sí cambia: es la de ejemplo de la nueva configuración.
        assert_eq!(
            cursor.history[0].hands()[0],
            Cursor::example_hand(HandConfig {
                pares: true,
                juego: false
            })
        );
    }

    /// Con una cola de movimientos sin jugar, `strategies()` tiene que seguir respondiendo por el
    /// nodo actual.
    #[test]
    fn strategies_works_with_a_dangling_tail() {
        let vacio = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        let entries = tree_entries(&vacio, 0, &[]);
        let mut cursor = write_cursor(GameType::MusGameTwoPlayers, 0, entries);
        while cursor.phase().is_some() && cursor.act(CursorMove::Play(Accion::Paso)).is_ok() {}

        let sin_jugadas = HandConfig {
            pares: false,
            juego: false,
        };
        cursor
            .set_hand_configs(&[sin_jugadas, sin_jugadas])
            .unwrap();
        assert!(
            cursor.moves.len() > cursor.history_len() - 1,
            "hay cola sin jugar"
        );

        cursor.seek(0);
        assert!(!cursor.strategies().unwrap().is_empty());
    }

    /// Si la nueva configuración acorta la secuencia de lances, la cola del registro deja de ser
    /// jugable pero no se borra: deshacer el cambio devuelve la línea entera.
    #[test]
    fn a_shorter_lance_sequence_is_reversible() {
        let mut cursor = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        while cursor.phase().is_some() && cursor.act(CursorMove::Play(Accion::Paso)).is_ok() {}
        // Con pares y juego se juegan grande, chica, pares y juego.
        assert_eq!(cursor.moves.len(), 8);
        assert_eq!(cursor.history_len(), 9);

        let con_jugadas = HandConfig {
            pares: true,
            juego: true,
        };
        let sin_jugadas = HandConfig {
            pares: false,
            juego: false,
        };
        cursor
            .set_hand_configs(&[sin_jugadas, sin_jugadas])
            .unwrap();
        // Sin pares ni juego quedan grande, chica y punto: dos movimientos dejan de jugarse.
        assert_eq!(cursor.moves.len(), 8, "el registro se conserva");
        assert_eq!(cursor.history_len(), 7);
        assert_eq!(cursor.position(), 6);

        cursor
            .set_hand_configs(&[con_jugadas, con_jugadas])
            .unwrap();
        assert_eq!(cursor.history_len(), 9, "la línea vuelve entera");
    }

    /// Cambiar las manos de una en una tiene que dar lo mismo que cambiarlas de golpe. Es lo que
    /// permite manejar la configuración con casillas de verificación, una por jugada y jugador.
    #[test]
    fn setting_hands_one_at_a_time_is_not_destructive() {
        let sin_jugadas = HandConfig {
            pares: false,
            juego: false,
        };
        let linea = |mut cursor: Cursor, de_golpe: bool| {
            while cursor.phase().is_some() && cursor.act(CursorMove::Play(Accion::Paso)).is_ok() {}
            if de_golpe {
                cursor
                    .set_hand_configs(&[sin_jugadas, sin_jugadas])
                    .unwrap();
            } else {
                // El estado intermedio tiene menos lances que el inicial y que el final: con solo
                // una mano con pares el lance no se juega.
                cursor.set_hand_config(0, sin_jugadas).unwrap();
                assert_eq!(cursor.history_len(), 5, "solo quedan grande y chica");
                cursor.set_hand_config(1, sin_jugadas).unwrap();
            }
            (cursor.moves.len(), cursor.history_len())
        };
        assert_eq!(
            linea(
                write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new()),
                false
            ),
            linea(
                write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new()),
                true
            )
        );
    }

    /// La apuesta máxima de un lance son los tantos que le quedan al equipo más rezagado
    /// (`crear_estado_lance`), y el filtrado por ella es común a todos los tipos de partida. Si un
    /// envite guardado deja de caber, el movimiento deja de jugarse, pero vuelve al restaurar los
    /// tantos.
    #[test]
    fn an_action_that_stops_being_legal_is_reversible() {
        for game_type in [GameType::MusGameTwoPlayers, GameType::MusGame] {
            let mut cursor = write_cursor(game_type, 0, HashMap::new());
            cursor.act(CursorMove::Play(Accion::Envido(10))).unwrap();
            assert_eq!(cursor.history_len(), 2);

            // Con [39, 0] al segundo equipo aún le quedan 40 tantos y el envite cabe.
            cursor.set_tantos([39, 0]);
            assert_eq!(cursor.history_len(), 2, "{game_type:?}");

            // Con [39, 39] la apuesta máxima baja a 1 y ya no cabe.
            cursor.set_tantos([39, 39]);
            let CursorNode::Play(actions) = cursor.cursor_node() else {
                panic!("se esperaba un nodo de jugador");
            };
            assert!(!actions.contains(&Accion::Envido(10)), "{actions:?}");
            assert_eq!(cursor.history_len(), 1, "{game_type:?}");
            assert_eq!(cursor.position(), 0);
            assert_eq!(cursor.moves.len(), 1, "el registro se conserva");

            cursor.set_tantos([0, 0]);
            assert_eq!(cursor.history_len(), 2, "la línea vuelve");
        }
    }

    #[test]
    fn set_tantos_keeps_the_line() {
        let mut cursor = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        cursor.act(CursorMove::Play(Accion::Envido(10))).unwrap();
        cursor.set_tantos([20, 7]);
        assert_eq!(cursor.moves.len(), 1);
        assert_eq!(cursor.position(), 1);
        assert_eq!(cursor.tantos, [20, 7]);
    }

    /// A partir de pares la jugada ya es pública y sí restringe las manos.
    #[test]
    fn pares_restricts_the_hands() {
        let mut cursor = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        while !matches!(cursor.phase(), Some(FasePartida::Envites(Lance::Pares))) {
            cursor.act(CursorMove::Play(Accion::Paso)).unwrap();
        }
        assert_eq!(
            cursor.declared(),
            HandConfig {
                pares: true,
                juego: false
            }
        );
        let con_pares = Mano::new([Carta::Rey; 4]);
        let sin_pares = Mano::new([Carta::Rey, Carta::Caballo, Carta::Sota, Carta::As]);
        assert!(cursor.accepts_hand(0, &con_pares));
        assert!(!cursor.accepts_hand(0, &sin_pares));
        assert!(matches!(
            cursor.strategy_for_hand(&HandKind::OneHand(sin_pares)),
            Ok(None)
        ));

        // El juego todavía no está declarado, así que no filtra: la mano de ejemplo tiene juego.
        let con_pares_sin_juego = Mano::new([Carta::As; 4]);
        assert!(cursor.hand_configs()[0].juego);
        assert!(cursor.accepts_hand(0, &con_pares_sin_juego));
    }

    /// La variante de `HandKind` tiene que corresponder al tipo de juego.
    #[test]
    fn strategy_for_hand_rejects_the_wrong_hand_count() {
        let reyes = Mano::new([Carta::Rey; 4]);
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        assert!(matches!(
            cursor.strategy_for_hand(&HandKind::TwoHands(reyes.clone(), reyes.clone())),
            Err(SolverError::WrongHandCount(1))
        ));

        let cursor = write_cursor(GameType::MusGameTwoHands, 0, HashMap::new());
        assert!(matches!(
            cursor.strategy_for_hand(&HandKind::OneHand(reyes)),
            Err(SolverError::WrongHandCount(2))
        ));
    }

    /// La probabilidad conjunta descuenta las cartas ya repartidas: no es el producto de las dos
    /// marginales.
    #[test]
    fn two_hands_prior_is_conditioned() {
        let independiente: f64 = DistribucionCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS)
            .map(|(_, prior)| prior)
            .sum::<f64>()
            .powi(2);
        let conjunta: f64 = DistribucionDobleCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS)
            .map(|(_, _, prior)| prior)
            .sum();
        assert!((independiente - 1.).abs() < 1e-9);
        assert!((conjunta - 1.).abs() < 1e-9);

        // Con ocho Reyes en la baraja, dos manos de cuatro Reyes son posibles pero menos
        // probables que el producto de sus marginales.
        let reyes = [Carta::Rey; 4];
        let marginal = DistribucionCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS)
            .find(|(cartas, _)| *cartas == reyes)
            .unwrap()
            .1;
        let conjunta = DistribucionDobleCartaIter::<4, 8>::new(Baraja::FREC_BARAJA_MUS)
            .find(|(c1, c2, _)| *c1 == reyes && *c2 == reyes)
            .unwrap()
            .2;
        assert!(conjunta > 0.);
        assert!(
            conjunta < marginal * marginal,
            "{conjunta} >= {marginal}^2: el reparto no descuenta cartas"
        );
    }

    /// Tras actuar, el peso incorpora la probabilidad de las acciones propias del jugador.
    #[test]
    fn reach_accumulates_the_players_own_actions() {
        // Sin ronda de mus la partida empieza en Grande, donde el jugador 0 tiene 5 acciones.
        let cursor = write_cursor(GameType::MusGameTwoPlayers, 0, HashMap::new());
        let moves = vec![
            CursorMove::Play(Accion::Paso),
            CursorMove::Play(Accion::Paso),
        ];
        let entries = tree_entries(&cursor, 0, &moves);
        let mut cursor = write_cursor(GameType::MusGameTwoPlayers, 0, entries);

        assert_eq!(cursor.strategies().unwrap()[0].actions().len(), 5);
        let en_raiz: Vec<f64> = cursor
            .strategies()
            .unwrap()
            .iter()
            .map(|s| s.reach_probability())
            .collect();

        for movimiento in moves {
            cursor.act(movimiento).unwrap();
        }
        let tras_actuar: Vec<f64> = cursor
            .strategies()
            .unwrap()
            .iter()
            .map(|s| s.reach_probability())
            .collect();

        assert_eq!(en_raiz.len(), tras_actuar.len());
        // El jugador 0 ha pasado una vez, con probabilidad 20/100 en un nodo de 5 acciones.
        for (raiz, actuado) in en_raiz.iter().zip(tras_actuar.iter()) {
            assert!(
                (actuado - raiz * 0.2).abs() < 1e-9,
                "{actuado} != {raiz} * 0.2"
            );
        }
    }
}
