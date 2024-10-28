use std::{
    cell::{Ref, RefCell},
    cmp::Ordering,
    fs::{self},
    path::{Path, PathBuf},
};

use serde::Deserialize;
use walkdir::WalkDir;

use crate::{
    mus::{Accion, Lance},
    Cfr,
};

use super::{GameConfig, SolverError, Strategy, StrategyConfig, TrainerConfig};

#[derive(Debug)]
pub struct BancoEstrategias {
    grande: RefCell<Cfr<Accion>>,
    chica: RefCell<Cfr<Accion>>,
    pares: RefCell<Cfr<Accion>>,
    juego: RefCell<Cfr<Accion>>,
    punto: RefCell<Cfr<Accion>>,
}

impl BancoEstrategias {
    pub fn new() -> Self {
        Self {
            grande: RefCell::new(Cfr::new()),
            chica: RefCell::new(Cfr::new()),
            pares: RefCell::new(Cfr::new()),
            juego: RefCell::new(Cfr::new()),
            punto: RefCell::new(Cfr::new()),
        }
    }

    pub fn estrategia_lance(&self, l: Lance) -> Ref<'_, Cfr<Accion>> {
        match l {
            Lance::Grande => self.grande.borrow(),
            Lance::Chica => self.chica.borrow(),
            Lance::Pares => self.pares.borrow(),
            Lance::Punto => self.punto.borrow(),
            Lance::Juego => self.juego.borrow(),
        }
    }
    pub fn estrategia_lance_mut(&self, l: Lance) -> &std::cell::RefCell<Cfr<Accion>> {
        match l {
            Lance::Grande => &self.grande,
            Lance::Chica => &self.chica,
            Lance::Pares => &self.pares,
            Lance::Punto => &self.punto,
            Lance::Juego => &self.juego,
        }
    }

    pub fn find(path: &Path) -> Vec<(String, StrategyConfig)> {
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
            if let Some(ext) = path.extension() {
                if ext != "json" {
                    continue;
                }
                let contents = match fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                #[derive(Debug, Deserialize)]
                struct MockStrategy {
                    strategy_config: StrategyConfig,
                }
                let mock_strategy: MockStrategy = match serde_json::from_str(&contents) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                result.push((path.display().to_string(), mock_strategy.strategy_config));
            }
        }
        result
    }

    pub fn load_estrategia(&self, path: &Path, l: Lance) -> Result<Strategy, SolverError> {
        let mut estrategia_path = PathBuf::from(path);
        estrategia_path.push(format!("{:?}", l));
        estrategia_path.set_extension("json");
        println!("Loading {:?}", estrategia_path);
        let strategy = Strategy::from_file(estrategia_path.as_path())?;
        // let cfr = self.estrategia_lance_mut(l).borrow_mut();
        // strategy.nodes.iter().for_each(|(info_set, probabilities)| {
        //     let node = Node::new(probabilities.len());
        //     cfr.nodes().insert(info_set.clone(), node);
        // });
        Ok(strategy)
    }

    pub fn export_estrategia(
        &self,
        path: &Path,
        l: Lance,
        trainer_config: &TrainerConfig,
        game_config: &GameConfig,
    ) -> Result<(), SolverError> {
        fs::create_dir_all(path).map_err(|err| {
            SolverError::NoCreateFolderPermission(err, path.display().to_string())
        })?;
        let mut estrategia_path = PathBuf::from(path);
        estrategia_path.push(format!("{:?}", l));
        estrategia_path.set_extension("json");
        let c = self.estrategia_lance(l);
        let strategy = Strategy::new(&c, trainer_config, game_config);
        strategy.to_file(estrategia_path.as_path())
    }

    pub fn export(
        &self,
        path: &Path,
        trainer_config: &TrainerConfig,
        game_config: &GameConfig,
    ) -> Result<(), SolverError> {
        self.export_estrategia(path, Lance::Grande, trainer_config, game_config)?;
        self.export_estrategia(path, Lance::Chica, trainer_config, game_config)?;
        self.export_estrategia(path, Lance::Punto, trainer_config, game_config)?;
        self.export_estrategia(path, Lance::Pares, trainer_config, game_config)?;
        self.export_estrategia(path, Lance::Juego, trainer_config, game_config)?;
        Ok(())
    }
}

impl Default for BancoEstrategias {
    fn default() -> Self {
        Self::new()
    }
}
