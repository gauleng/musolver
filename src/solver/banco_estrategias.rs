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

#[derive(Debug, Clone, Copy)]
pub enum TipoEstrategia {
    CuatroManos = 0,
    TresManos1vs2 = 1,
    TresManos1vs2Intermedio = 2,
    TresManos2vs1 = 3,
    DosManos = 4,
}

impl Display for TipoEstrategia {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TipoEstrategia::CuatroManos => write!(f, "2-2"),
            TipoEstrategia::TresManos1vs2 => write!(f, "1-2"),
            TipoEstrategia::TresManos1vs2Intermedio => write!(f, "1-1-1"),
            TipoEstrategia::TresManos2vs1 => write!(f, "2-1"),
            TipoEstrategia::DosManos => write!(f, "1-1"),
        }
    }
}

pub struct ManosNormalizadas<'a>([(&'a Mano, Option<&'a Mano>); 2]);

impl<'a> ManosNormalizadas<'a> {
    //// Manos de la pareja mano o postre según el parámetro recibido.
    pub fn manos(&self, p: usize) -> &(&'a Mano, Option<&'a Mano>) {
        &self.0[p]
    }

    fn par_manos_to_string(manos: &(&'a Mano, Option<&'a Mano>)) -> String {
        manos.0.to_string() + "," + &manos.1.map_or_else(|| "".to_owned(), |m| m.to_string())
    }

    fn mano_to_abstract_string(m: &Mano, l: &Lance) -> String {
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

    fn par_manos_to_abstract_string(manos: &(&'a Mano, Option<&'a Mano>), l: &Lance) -> String {
        Self::mano_to_abstract_string(manos.0, l)
            + ","
            + &manos
                .1
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

impl<'a> TipoEstrategia {
    pub fn normalizar_mano(m: &'a [Mano], l: &Lance) -> (Self, ManosNormalizadas<'a>) {
        match l {
            Lance::Grande | Lance::Chica | Lance::Punto => {
                let mn = [(&m[0], Some(&m[2])), (&m[1], Some(&m[3]))];
                (TipoEstrategia::CuatroManos, ManosNormalizadas(mn))
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
    ) -> (Self, ManosNormalizadas<'a>) {
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
            let mn = [(&m[0], Some(&m[2])), (&m[1], Some(&m[3]))];
            (TipoEstrategia::CuatroManos, ManosNormalizadas(mn))
        } else if parejas[0].len() == 1 && parejas[1].len() == 1 {
            let mn = [(parejas[0][0], None), (parejas[1][0], None)];
            (TipoEstrategia::DosManos, ManosNormalizadas(mn))
        } else if parejas[0].len() == 1 && parejas[1].len() == 2 {
            let tipo_estrategia = if jugadas[2].is_none() {
                TipoEstrategia::TresManos1vs2
            } else {
                TipoEstrategia::TresManos1vs2Intermedio
            };
            let mn = [(parejas[0][0], None), (parejas[1][0], Some(parejas[1][1]))];
            (tipo_estrategia, ManosNormalizadas(mn))
        } else {
            let mn = [(parejas[0][0], Some(parejas[0][1])), (parejas[1][0], None)];
            (TipoEstrategia::TresManos2vs1, ManosNormalizadas(mn))
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
        trainer_config: &TrainerConfig,
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
        trainer_config: &TrainerConfig,
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
