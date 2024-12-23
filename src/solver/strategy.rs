use std::{
    collections::HashMap,
    fs::{self},
    iter::zip,
    path::Path,
};

use serde::{Deserialize, Serialize};

use crate::{
    mus::{Accion, Lance, Mano, PartidaMus},
    ActionNode, Cfr, Game,
};

use super::{LanceGame, SolverError, TrainerConfig};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub lance: Option<Lance>,
    pub abstract_game: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrategyConfig {
    pub trainer_config: TrainerConfig,
    pub game_config: GameConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Strategy<G: Game<P = usize, A = Accion>> {
    pub strategy_config: StrategyConfig,
    pub nodes: HashMap<String, (Vec<G::A>, Vec<f64>)>,
}

impl<G: Game<P = usize, A = Accion> + Clone> Strategy<G> {
    pub fn new(cfr: &Cfr<G>, trainer_config: &TrainerConfig, game_config: &GameConfig) -> Self {
        let nodes = cfr
            .nodes()
            .iter()
            .map(|(info_set, node)| {
                let actions = zip(node.actions().to_owned(), node.get_average_strategy()).collect();
                (info_set.to_owned(), actions)
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

    pub fn best_response_value(
        &self,
        hand1: &Mano,
        hand2: &Mano,
        action_node: &ActionNode<usize, Accion>,
        history: &[Accion],
        player: usize,
        opponent_hands: &[(Mano, Mano, f64)],
    ) -> f64 {
        match action_node {
            ActionNode::Terminal => {
                let opponent_dist_total: f64 = opponent_hands.iter().map(|(_, _, p)| p).sum();
                let mut expected_payoff = 0.;
                for (opponent_hand1, opponent_hand2, probability) in opponent_hands {
                    let opponent_dist = probability / opponent_dist_total;
                    let hands = [
                        hand1.clone(),
                        opponent_hand1.clone(),
                        hand2.clone(),
                        opponent_hand2.clone(),
                    ];
                    let mut lance_game = LanceGame::from_partida_mus(
                        &PartidaMus::new_partida_lance(
                            self.strategy_config.game_config.lance.unwrap(),
                            hands,
                            [0, 0],
                        )
                        .unwrap(),
                        false,
                    );
                    if let Some(l) = &mut lance_game {
                        expected_payoff += opponent_dist * l.utility(player);
                    }
                }
                expected_payoff
            }
            ActionNode::NonTerminal(acting_player, children) => {
                let mut new_opponent_hands = opponent_hands.to_owned();
                let mut weights = vec![0.; children.len()];
                let mut util = vec![0.; children.len()];
                let mut max_util = 0.;
                for (idx_action, (action, next_node)) in children.iter().enumerate() {
                    if player != *acting_player {
                        for (idx_hands, (opponent_hand1, opponent_hand2, prob)) in
                            opponent_hands.iter().enumerate()
                        {
                            let hands = [
                                hand1.clone(),
                                opponent_hand1.clone(),
                                hand2.clone(),
                                opponent_hand2.clone(),
                            ];
                            let lance_game = LanceGame::from_partida_mus(
                                &PartidaMus::new_partida_lance(
                                    self.strategy_config.game_config.lance.unwrap(),
                                    hands,
                                    [0, 0],
                                )
                                .unwrap(),
                                self.strategy_config.game_config.abstract_game,
                            )
                            .unwrap();
                            let info_set_str = lance_game.info_set_str(*acting_player);
                            let strategy = self.nodes.get(&info_set_str).unwrap();
                            new_opponent_hands[idx_hands].2 = prob * strategy.1[idx_action];
                            weights[idx_action] += new_opponent_hands[idx_hands].2;
                        }
                    }
                    let mut new_history = history.to_vec();
                    new_history.push(*action);
                    util[idx_action] = self.best_response_value(
                        hand1,
                        hand2,
                        next_node,
                        &new_history,
                        player,
                        &new_opponent_hands,
                    );
                    if player == *acting_player && util[idx_action] > max_util {
                        max_util = util[idx_action];
                    }
                }
                if player != *acting_player {
                    let sum_weights: f64 = weights.iter().sum();
                    let normalized_weights = weights.iter().map(|w| w / sum_weights);
                    max_util = zip(util.iter(), normalized_weights)
                        .map(|(u, w)| u * w)
                        .sum();
                }
                max_util
            }
        }
    }

    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), SolverError> {
        let contents = serde_json::to_string(self).map_err(SolverError::ParseStrategyJsonError)?;
        fs::write(path.as_ref(), contents).map_err(|err| {
            SolverError::InvalidStrategyPath(err, path.as_ref().display().to_string())
        })?;
        Ok(())
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, SolverError> {
        let contents = fs::read_to_string(path.as_ref()).map_err(|err| {
            SolverError::InvalidStrategyPath(err, path.as_ref().display().to_string())
        })?;
        let n: Self =
            serde_json::from_str(&contents).map_err(SolverError::ParseStrategyJsonError)?;
        Ok(n)
    }
}
