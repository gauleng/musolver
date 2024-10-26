use std::{
    collections::HashMap,
    fs::{self, File},
    iter::zip,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    mus::{Accion, Lance, Mano, PartidaMus},
    ActionNode, Cfr, Game,
};

use super::{LanceGameDosManos, SolverError, TrainerConfig};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub lance: Option<Lance>,
    pub abstract_game: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrategyConfig {
    pub trainer_config: TrainerConfig<usize, Accion>,
    pub game_config: GameConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Strategy {
    pub strategy_config: StrategyConfig,
    pub nodes: HashMap<String, Vec<f64>>,
}

impl Strategy {
    pub fn new(
        cfr: &Cfr,
        trainer_config: &TrainerConfig<usize, Accion>,
        game_config: &GameConfig,
    ) -> Self {
        let nodes = cfr
            .nodes()
            .iter()
            .map(|(info_set, node)| (info_set.to_owned(), node.get_average_strategy()))
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
                    let lance_game = LanceGameDosManos::from_partida_mus(
                        &PartidaMus::new_partida_lance(
                            self.strategy_config.game_config.lance.unwrap(),
                            hands,
                            [0, 0],
                        )
                        .unwrap(),
                        false,
                    );
                    if let Some(l) = lance_game {
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
                            let lance_game = LanceGameDosManos::from_partida_mus(
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
                            new_opponent_hands[idx_hands].2 = prob * strategy[idx_action];
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

    pub fn to_file(&self, path: &Path) -> Result<(), SolverError> {
        let contents = serde_json::to_string(self).map_err(SolverError::StrategyParseJsonError)?;
        fs::write(path, contents)
            .map_err(|err| SolverError::InvalidStrategyPath(err, path.display().to_string()))?;
        Ok(())
    }

    pub fn from_file(path: &Path) -> Result<Self, SolverError> {
        let contents = fs::read_to_string(path)
            .map_err(|err| SolverError::InvalidStrategyPath(err, path.display().to_string()))?;
        let n: Self =
            serde_json::from_str(&contents).map_err(SolverError::StrategyParseJsonError)?;
        Ok(n)
    }

    fn lance_to_filename(l: Option<Lance>) -> String {
        match l {
            Some(lance) => format!("{:?}", lance),
            None => "Mus".to_string(),
        }
    }

    pub fn to_csv(&self, path: &Path) -> std::io::Result<()> {
        let mut csv_path = PathBuf::from(path);
        csv_path.push(Strategy::lance_to_filename(
            self.strategy_config.game_config.lance,
        ));
        csv_path.set_extension("csv");
        let file = File::create(csv_path)?;
        let mut wtr = csv::WriterBuilder::new()
            .flexible(true)
            .quote_style(csv::QuoteStyle::Never)
            .from_writer(&file);

        let mut nodes_vec: Vec<(String, Vec<f64>)> = self
            .nodes
            .iter()
            .map(|(s, n)| (s.clone(), n.clone()))
            .collect();
        nodes_vec.sort_by(|x, y| x.0.cmp(&y.0));
        for (k, n) in nodes_vec {
            let mut probabilities: Vec<String> = n.iter().map(|f| f.to_string()).collect();
            probabilities.insert(0, k);
            wtr.write_record(&probabilities)?;
        }
        wtr.flush()?;

        let mut config_path = PathBuf::from(path);
        config_path.push("config");
        config_path.set_extension("json");
        let contents = serde_json::to_string(&self.strategy_config)?;
        fs::write(config_path, contents)?;
        Ok(())
    }

    pub fn from_csv(path: &Path) -> std::io::Result<Self> {
        let mut config_path = PathBuf::from(path);
        config_path.push("config");
        config_path.set_extension("json");
        let contents = fs::read_to_string(config_path)?;
        let strategy_config: StrategyConfig = serde_json::from_str(&contents)?;

        let mut csv_path = PathBuf::from(path);
        csv_path.push(Strategy::lance_to_filename(
            strategy_config.game_config.lance,
        ));
        csv_path.set_extension("csv");
        let file = File::open(path)?;
        let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(file);
        let mut nodes = HashMap::new();
        for result in rdr.records() {
            let record = result?;
            let info_set = format!(
                "{},{},{},{}",
                &record[0], &record[1], &record[2], &record[3]
            );
            let probabilities: Result<Vec<f64>, std::num::ParseFloatError> =
                record.iter().skip(4).map(|p| p.parse::<f64>()).collect();
            if let Ok(p) = probabilities {
                nodes.insert(info_set, p);
            }
        }

        Ok(Self {
            nodes,
            strategy_config,
        })
    }
}
