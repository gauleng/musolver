use std::{fs, path::Path};

use clap::ValueEnum;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::{
    mus::{Accion, Lance},
    ActionNode, Cfr, Game,
};

pub enum Trainer {
    LanceTrainer(Lance),
    MusTrainer,
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
pub enum CfrMethod {
    Cfr,
    CfrPlus,
    ChanceSampling,
    ExternalSampling,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrainerConfig<A> {
    pub method: CfrMethod,
    pub iterations: usize,
    pub action_tree: ActionNode<usize, A>,
}

impl<A> TrainerConfig<A>
where
    A: Serialize + for<'a> Deserialize<'a>,
{
    pub fn to_file(&self, path: &Path) {
        let contents = serde_json::to_string(self).expect("Error converting to JSON");
        fs::write(path, contents).expect("Error writing config");
    }

    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let n: Self = serde_json::from_str(&contents).unwrap();

        Ok(n)
    }
}

impl Trainer {
    pub fn train<G, A>(&self, cfr: &mut Cfr<A>, game: &mut G, config: &TrainerConfig<A>)
    where
        G: Game<usize, A>,
        A: Eq + Copy,
    {
        use std::time::Instant;

        let now = Instant::now();
        let pb = ProgressBar::new(config.iterations as u64);
        pb.set_style(
            ProgressStyle::with_template("{wide_bar:40.cyan/blue} {human_pos}/{human_len} {msg} ")
                .unwrap()
                .progress_chars("##-"),
        );
        let mut util = [0., 0.];
        for i in 0..config.iterations {
            game.new_random();
            match config.method {
                CfrMethod::Cfr => todo!(),
                CfrMethod::CfrPlus => todo!(),
                CfrMethod::ChanceSampling => {
                    util[0] += cfr.chance_cfr(game, &config.action_tree, 0, 1., 1.);
                    util[1] += cfr.chance_cfr(game, &config.action_tree, 1, 1., 1.);
                }
                CfrMethod::ExternalSampling => {
                    util[0] += cfr.external_cfr(game, &config.action_tree, 0);
                    util[1] += cfr.external_cfr(game, &config.action_tree, 1);
                }
            }

            pb.inc(1);
            if i % 1000 == 0 {
                pb.set_message(format!(
                    "Utility: {:.5} {:.5}",
                    util[0] / (i as f64),
                    util[1] / (i as f64),
                ));
            }
            // if i % 100000000 == 0 {
            //     banco
            //         .export_estrategia_lance(lance)
            //         .expect("Error exportando estrategias.");
            // }
        }
        let elapsed = now.elapsed();
        println!("Elapsed: {:.2?}", elapsed);
    }
}
