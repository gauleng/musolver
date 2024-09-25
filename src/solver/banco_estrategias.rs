use std::{
    cell::{Ref, RefCell},
    fmt::Display,
    fs::{self, File},
};

use crate::{
    mus::{Juego, Lance, Mano, Pares},
    Cfr, Node,
};

#[derive(Debug, Clone, Copy)]
pub enum TipoEstrategia {
    CuatroManos = 0,
    TresManos1vs2 = 1,
    TresManos1vs2Intermedio = 2,
    TresManos2vs1 = 3,
    DosManos = 4,
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

#[derive(Debug)]
pub struct BancoEstrategias {
    grande: Vec<RefCell<Cfr>>,
    chica: Vec<RefCell<Cfr>>,
    pares: Vec<RefCell<Cfr>>,
    juego: Vec<RefCell<Cfr>>,
    punto: Vec<RefCell<Cfr>>,
}

impl BancoEstrategias {
    pub fn new() -> Self {
        Self {
            grande: vec![RefCell::new(Cfr::new())],
            chica: vec![RefCell::new(Cfr::new())],
            pares: vec![RefCell::new(Cfr::new()); 5],
            juego: vec![RefCell::new(Cfr::new()); 5],
            punto: vec![RefCell::new(Cfr::new())],
        }
    }

    pub fn estrategia_lance(&self, l: Lance, t: TipoEstrategia) -> Ref<'_, Cfr> {
        match l {
            Lance::Grande => self.grande[0].borrow(),
            Lance::Chica => self.chica[0].borrow(),
            Lance::Pares => self.pares[t as usize].borrow(),
            Lance::Punto => self.punto[0].borrow(),
            Lance::Juego => self.juego[t as usize].borrow(),
        }
    }
    pub fn estrategia_lance_mut(&self, l: Lance, t: TipoEstrategia) -> &std::cell::RefCell<Cfr> {
        match l {
            Lance::Grande => &self.grande[0],
            Lance::Chica => &self.chica[0],
            Lance::Pares => &self.pares[t as usize],
            Lance::Punto => &self.punto[0],
            Lance::Juego => &self.juego[t as usize],
        }
    }

    fn export_estrategia(&self, path: &str, l: Lance, t: TipoEstrategia) -> std::io::Result<()> {
        fs::create_dir_all(path)?;
        let file_name = format!("{path}/{:?}_{:?}.csv", l, t);
        let file = File::create(file_name)?;
        let mut wtr = csv::WriterBuilder::new()
            .flexible(true)
            .quote_style(csv::QuoteStyle::Never)
            .from_writer(&file);
        let c = self.estrategia_lance(l, t);

        let mut v: Vec<(String, Node)> = c
            .nodes()
            .iter()
            .map(|(s, n)| (s.clone(), n.clone()))
            .collect();
        v.sort_by(|x, y| x.0.cmp(&y.0));
        for (k, n) in v {
            let mut probabilities = n
                .get_average_strategy()
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<String>>();
            probabilities.insert(0, k);
            wtr.write_record(&probabilities)?;
        }
        wtr.flush()?;
        Ok(())
    }

    pub fn export_estrategia_lance(&self, path: &str, l: Lance) -> std::io::Result<()> {
        match l {
            Lance::Grande => {
                self.export_estrategia(path, Lance::Grande, TipoEstrategia::CuatroManos)?;
            }
            Lance::Chica => {
                self.export_estrategia(path, Lance::Chica, TipoEstrategia::CuatroManos)?;
            }
            Lance::Pares => {
                self.export_estrategia(path, Lance::Pares, TipoEstrategia::CuatroManos)?;
                self.export_estrategia(path, Lance::Pares, TipoEstrategia::TresManos2vs1)?;
                self.export_estrategia(path, Lance::Pares, TipoEstrategia::TresManos1vs2)?;
                self.export_estrategia(
                    path,
                    Lance::Pares,
                    TipoEstrategia::TresManos1vs2Intermedio,
                )?;
                self.export_estrategia(path, Lance::Pares, TipoEstrategia::DosManos)?;
            }
            Lance::Punto => {
                self.export_estrategia(path, Lance::Punto, TipoEstrategia::CuatroManos)?;
            }
            Lance::Juego => {
                self.export_estrategia(path, Lance::Juego, TipoEstrategia::CuatroManos)?;
                self.export_estrategia(path, Lance::Juego, TipoEstrategia::TresManos2vs1)?;
                self.export_estrategia(path, Lance::Juego, TipoEstrategia::TresManos1vs2)?;
                self.export_estrategia(
                    path,
                    Lance::Juego,
                    TipoEstrategia::TresManos1vs2Intermedio,
                )?;
                self.export_estrategia(path, Lance::Juego, TipoEstrategia::DosManos)?;
            }
        }
        Ok(())
    }

    pub fn export(&self, path: &str) -> std::io::Result<()> {
        self.export_estrategia_lance(path, Lance::Grande)?;
        self.export_estrategia_lance(path, Lance::Chica)?;
        self.export_estrategia_lance(path, Lance::Punto)?;
        self.export_estrategia_lance(path, Lance::Pares)?;
        self.export_estrategia_lance(path, Lance::Juego)?;
        Ok(())
    }
}

impl Default for BancoEstrategias {
    fn default() -> Self {
        Self::new()
    }
}
