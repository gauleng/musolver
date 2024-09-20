use std::{
    cell::{Ref, RefCell},
    fs::File,
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

impl TipoEstrategia {
    pub fn normalizar_mano(m: &[Mano], l: &Lance) -> (Self, [String; 2]) {
        match l {
            Lance::Grande | Lance::Chica | Lance::Punto => {
                let m1 = m[0].to_string() + "," + &m[2].to_string();
                let m2 = m[1].to_string() + "," + &m[3].to_string();
                (TipoEstrategia::CuatroManos, [m1, m2])
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

    fn normalizar_mano_jugadas<T>(m: &[Mano], jugadas: &[Option<T>]) -> (Self, [String; 2]) {
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
            let m1 = m[0].to_string() + "," + &m[2].to_string();
            let m2 = m[1].to_string() + "," + &m[3].to_string();
            (TipoEstrategia::CuatroManos, [m1, m2])
        } else if parejas[0].len() == 1 && parejas[1].len() == 1 {
            (
                TipoEstrategia::DosManos,
                [
                    parejas[0][0].to_string() + ",",
                    parejas[1][0].to_string() + ",",
                ],
            )
        } else if parejas[0].len() == 1 && parejas[1].len() == 2 {
            let tipo_estrategia = if jugadas[2].is_none() {
                TipoEstrategia::TresManos1vs2
            } else {
                TipoEstrategia::TresManos1vs2Intermedio
            };
            (
                tipo_estrategia,
                [
                    parejas[0][0].to_string() + ",",
                    parejas[1][0].to_string() + "," + &parejas[1][1].to_string(),
                ],
            )
        } else {
            (
                TipoEstrategia::TresManos2vs1,
                [
                    parejas[0][0].to_string() + "," + &parejas[0][1].to_string(),
                    parejas[1][0].to_string() + ",",
                ],
            )
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

    fn export_estrategia(&self, l: Lance, t: TipoEstrategia) -> std::io::Result<()> {
        let file_name = format!("{:?}_{:?}.csv", l, t);
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

    pub fn export_estrategia_lance(&self, l: Lance) -> std::io::Result<()> {
        match l {
            Lance::Grande => {
                self.export_estrategia(Lance::Grande, TipoEstrategia::CuatroManos)?;
            }
            Lance::Chica => {
                self.export_estrategia(Lance::Chica, TipoEstrategia::CuatroManos)?;
            }
            Lance::Pares => {
                self.export_estrategia(Lance::Pares, TipoEstrategia::CuatroManos)?;
                self.export_estrategia(Lance::Pares, TipoEstrategia::TresManos2vs1)?;
                self.export_estrategia(Lance::Pares, TipoEstrategia::TresManos1vs2)?;
                self.export_estrategia(Lance::Pares, TipoEstrategia::TresManos1vs2Intermedio)?;
                self.export_estrategia(Lance::Pares, TipoEstrategia::DosManos)?;
            }
            Lance::Punto => {
                self.export_estrategia(Lance::Punto, TipoEstrategia::CuatroManos)?;
            }
            Lance::Juego => {
                self.export_estrategia(Lance::Juego, TipoEstrategia::CuatroManos)?;
                self.export_estrategia(Lance::Juego, TipoEstrategia::TresManos2vs1)?;
                self.export_estrategia(Lance::Juego, TipoEstrategia::TresManos1vs2)?;
                self.export_estrategia(Lance::Juego, TipoEstrategia::TresManos1vs2Intermedio)?;
                self.export_estrategia(Lance::Juego, TipoEstrategia::DosManos)?;
            }
        }
        Ok(())
    }

    pub fn export(&self) -> std::io::Result<()> {
        self.export_estrategia_lance(Lance::Grande)?;
        self.export_estrategia_lance(Lance::Chica)?;
        self.export_estrategia_lance(Lance::Punto)?;
        self.export_estrategia_lance(Lance::Pares)?;
        self.export_estrategia_lance(Lance::Juego)?;
        Ok(())
    }
}

impl Default for BancoEstrategias {
    fn default() -> Self {
        Self::new()
    }
}
