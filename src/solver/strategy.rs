use std::{
    collections::HashMap,
    fs::{self, File},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    mus::{Accion, Lance},
    Cfr,
};

use super::TrainerConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfig {
    pub lance: Option<Lance>,
    pub abstract_game: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrategyConfig {
    pub trainer_config: TrainerConfig<Accion>,
    pub game_config: GameConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Strategy {
    pub strategy_config: StrategyConfig,
    pub nodes: HashMap<String, Vec<f64>>,
}

impl Strategy {
    pub fn new(
        cfr: &Cfr<Accion>,
        trainer_config: &TrainerConfig<Accion>,
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

    pub fn to_file(&self, path: &Path) -> std::io::Result<()> {
        let contents = serde_json::to_string(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let n: Self = serde_json::from_str(&contents).unwrap();
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
