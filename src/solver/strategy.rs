use std::{
    cmp::Ordering,
    collections::HashMap,
    fs::{self, File},
    path::Path,
    sync::Arc,
};

use arrayvec::ArrayVec;
use memmap2::Mmap;
use walkdir::WalkDir;

use crate::{
    Cfr, Game, NodeType,
    mus::{Accion, Carta, FasePartida, Lance, Mano, Turno},
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

pub struct Cursor {
    reader: Arc<StrategyReader>,
    history: Vec<Box<dyn GenericMus>>,
    actions: Vec<Accion>,
    position: usize,
    tantos: [u8; 2],
    pares: [bool; 4],
    juego: [bool; 4],
}

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

pub enum CursorNode {
    Play(Vec<Accion>),
    Discard,
    Terminal,
}

pub enum CursorMove<'a> {
    Play(Accion),
    Discard(DiscardAction<'a>),
}

pub enum DiscardAction<'a> {
    Count(usize),
    Cards(&'a [Carta]),
}

impl Cursor {
    fn new(reader: Arc<StrategyReader>) -> Self {
        let tantos = [0; 2];
        let pares = [true; 4];
        let juego = [true; 4];
        let game = Self::init_game(tantos, &reader.strategy_config.game_config, &pares, &juego);
        Cursor {
            history: vec![game],
            reader,
            tantos,
            pares,
            juego,
            position: 0,
            actions: vec![],
        }
    }

    pub fn set_tantos(&mut self, tantos: [u8; 2]) {
        self.tantos = tantos;
        self.history.clear();
        self.history.push(Self::init_game(
            self.tantos,
            &self.reader.strategy_config.game_config,
            &self.pares,
            &self.juego,
        ));
    }

    pub fn set_pares(&mut self, pares: &[bool]) {}

    pub fn set_juego(&mut self, juego: &[bool]) {}

    pub fn act(&mut self, action: CursorMove) -> Result<(), SolverError> {
        let game = &self.history[self.position()];
        let mut new_game = game.clone();
        new_game.act_with_action(action)?;
        self.history.push(new_game);
        Ok(())
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn strategy_for_hand(&self, mano: &HandKind) -> Result<Option<HandStrategy>, SolverError> {
        todo!();
    }

    pub fn strategies(&self) -> Result<Vec<HandStrategy>, SolverError> {
        todo!();
    }

    pub fn strategies_with_kept(&self, kept: &HandKind) -> Result<Vec<HandStrategy>, SolverError> {
        todo!();
    }

    pub fn cursor_node(&self) -> CursorNode {
        let game = &self.history[self.position()];
        match game.phase() {
            Some(phase) => match phase {
                FasePartida::Mus | FasePartida::Envites(_) => {
                    CursorNode::Play(game.actions().to_vec())
                }
                FasePartida::Descartes => CursorNode::Discard,
                FasePartida::DescartePendiente => todo!(),
            },
            None => CursorNode::Terminal,
        }
    }

    pub fn turn(&self) -> Option<Turno> {
        //self.history[self.position]
        None
    }

    pub fn phase(&self) -> Option<FasePartida> {
        todo!();
    }

    pub fn seek(&mut self, new_position: usize) {
        self.position = new_position.min(self.history.len())
    }

    pub fn position(&self) -> usize {
        self.position
    }

    fn init_game(
        tantos: [u8; 2],
        game_config: &GameConfig,
        pares: &[bool],
        juego: &[bool],
    ) -> Box<dyn GenericMus> {
        let manos: Vec<Mano> = std::iter::zip(pares, juego)
            .map(|(pares, juego)| Self::example_hand(*pares, *juego))
            .collect();
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

    fn example_hand(pares: bool, juego: bool) -> Mano {
        match (pares, juego) {
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
