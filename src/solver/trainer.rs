use std::fmt::Debug;

use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::{mus::Lance, ActionNode, Cfr, CfrMethod, Game};

pub enum Trainer {
    LanceTrainer(Lance),
    MusTrainer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrainerConfig<P, A> {
    pub method: CfrMethod,
    pub iterations: usize,
    pub action_tree: ActionNode<P, A>,
}

impl Trainer {
    pub fn train<G, P, A>(&self, cfr: &mut Cfr<A>, game: &mut G, config: &TrainerConfig<P, A>)
    where
        G: Game<P, A> + Debug,
        A: Eq + Copy,
        P: Eq + Copy,
    {
        use std::time::Instant;

        let now = Instant::now();
        let pb = ProgressBar::new(config.iterations as u64);
        pb.set_style(
            ProgressStyle::with_template("{wide_bar:40.cyan/blue} {human_pos}/{human_len} {msg} ")
                .unwrap()
                .progress_chars("##-"),
        );

        cfr.train(
            game,
            &config.action_tree,
            config.method,
            config.iterations,
            |i, util| {
                pb.inc(1);
                if i % 1000 == 0 {
                    pb.set_message(format!("Utility: {:.5} {:.5}", util[0], util[1],));
                }
            },
        );

        // if i % 100000000 == 0 {
        //     banco
        //         .export_estrategia_lance(lance)
        //         .expect("Error exportando estrategias.");
        // }
        let elapsed = now.elapsed();
        println!("Elapsed: {:.2?}", elapsed);
    }
}
