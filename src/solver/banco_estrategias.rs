use std::fs::File;

use crate::{mus::Lance, Cfr, Node};

use super::TipoEstrategia;

#[derive(Debug)]
pub struct BancoEstrategias {
    grande: Vec<Cfr>,
    chica: Vec<Cfr>,
    pares: Vec<Cfr>,
    juego: Vec<Cfr>,
    punto: Vec<Cfr>,
}

impl BancoEstrategias {
    pub fn new() -> Self {
        Self {
            grande: vec![Cfr::new()],
            chica: vec![Cfr::new()],
            pares: vec![Cfr::new(); 5],
            juego: vec![Cfr::new(); 5],
            punto: vec![Cfr::new()],
        }
    }

    pub fn estrategia_lance(&self, l: Lance, t: TipoEstrategia) -> &Cfr {
        match l {
            Lance::Grande => &self.grande[0],
            Lance::Chica => &self.chica[0],
            Lance::Pares => &self.pares[t as usize],
            Lance::Punto => &self.punto[0],
            Lance::Juego => &self.juego[t as usize],
        }
    }
    pub fn estrategia_lance_mut(&mut self, l: Lance, t: TipoEstrategia) -> &mut Cfr {
        match l {
            Lance::Grande => &mut self.grande[0],
            Lance::Chica => &mut self.chica[0],
            Lance::Pares => &mut self.pares[t as usize],
            Lance::Punto => &mut self.punto[0],
            Lance::Juego => &mut self.juego[t as usize],
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
