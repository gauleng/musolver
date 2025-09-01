use std::fmt::Debug;

use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::{mus::Lance, Cfr, CfrMethod, Game};

pub enum Trainer {
    LanceTrainer(Lance),
    MusTrainer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrainerConfig {
    pub method: CfrMethod,
    pub iterations: usize,
}

impl Trainer {
    pub fn train<G>(&self, cfr: &mut Cfr<G>, game: &mut G, config: &TrainerConfig)
    where
        G: Game + Debug + Clone,
        G::Action: Eq + Copy,
    {
        use std::time::Instant;

        let now = Instant::now();
        let pb = ProgressBar::new(config.iterations as u64);
        pb.set_style(
            ProgressStyle::with_template("{wide_bar:40.cyan/blue} {human_pos}/{human_len} {msg} ")
                .unwrap()
                .progress_chars("##-"),
        );

        cfr.train(game, config.method, config.iterations, |i, util| {
            pb.inc(1);
            if i % 1000 == 0 {
                pb.set_message(format!(
                    "Utility: {}",
                    util.iter()
                        .map(|u| format!("{u:.5}"))
                        .collect::<Vec<String>>()
                        .join(" "),
                ));
            }
        });

        // if i % 100000000 == 0 {
        //     banco
        //         .export_estrategia_lance(lance)
        //         .expect("Error exportando estrategias.");
        // }
        let elapsed = now.elapsed();
        println!("Elapsed: {elapsed:.2?}");
    }
}
