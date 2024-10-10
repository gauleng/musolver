use std::{
    cell::{Ref, RefCell},
    fmt::Display,
    fs::{self},
    path::{Path, PathBuf},
};

use crate::{
    mus::{Accion, Juego, Lance, Mano, Pares},
    Cfr,
};

use super::{GameConfig, Strategy, TrainerConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandConfiguration {
    CuatroManos,
    TresManos1vs2,
    TresManos1vs2Intermedio,
    TresManos2vs1,
    DosManos,
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

pub struct ManosNormalizadas([(Mano, Option<Mano>); 2]);

impl ManosNormalizadas {
    //// Manos de la pareja mano o postre según el parámetro recibido.
    pub fn manos(&self, p: usize) -> &(Mano, Option<Mano>) {
        &self.0[p]
    }

    pub fn par_manos_to_string(manos: &(Mano, Option<Mano>)) -> String {
        manos.0.to_string()
            + ","
            + &manos
                .1
                .as_ref()
                .map_or_else(|| "".to_owned(), |m| m.to_string())
    }

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

    pub fn par_manos_to_abstract_string(manos: &(Mano, Option<Mano>), l: &Lance) -> String {
        Self::mano_to_abstract_string(&manos.0, l)
            + ","
            + &manos
                .1
                .as_ref()
                .map_or_else(|| "".to_string(), |m| Self::mano_to_abstract_string(m, l))
    }

    pub fn to_string_array(&self) -> [String; 2] {
        [
            Self::par_manos_to_string(self.manos(0)),
            Self::par_manos_to_string(self.manos(1)),
        ]
    }

    pub fn to_abstract_string_array(&self, l: &Lance) -> [String; 2] {
        [
            Self::par_manos_to_abstract_string(self.manos(0), l),
            Self::par_manos_to_abstract_string(self.manos(1), l),
        ]
    }
}

impl<'a> HandConfiguration {
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

    pub fn load_estrategia(&self, path: &Path, l: Lance) -> std::io::Result<Strategy> {
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
        trainer_config: &TrainerConfig<usize, Accion>,
        game_config: &GameConfig,
    ) -> std::io::Result<()> {
        fs::create_dir_all(path)?;
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
        trainer_config: &TrainerConfig<usize, Accion>,
        game_config: &GameConfig,
    ) -> std::io::Result<()> {
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
