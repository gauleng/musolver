use std::{fmt::Debug, fs, path::Path};

use clap::ValueEnum;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::{mus::Lance, ActionNode, Cfr, Game};

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
        G: Game<usize, A> + Debug,
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
        let mut util = vec![0.; game.num_players()];
        for i in 0..config.iterations {
            for (player_idx, u) in util.iter_mut().enumerate() {
                let player_id = game.player_id(player_idx);
                match config.method {
                    CfrMethod::Cfr => {
                        game.new_iter(|game, po| {
                            *u += po * cfr.chance_cfr(game, &config.action_tree, player_id, 1., po);
                            pb.inc(1);
                        });
                    }
                    CfrMethod::CfrPlus => todo!(),
                    CfrMethod::ChanceSampling => {
                        game.new_random();
                        *u += cfr.chance_cfr(game, &config.action_tree, player_id, 1., 1.);
                    }
                    CfrMethod::ExternalSampling => {
                        game.new_random();
                        *u += cfr.external_cfr(game, &config.action_tree, player_id);
                    }
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
