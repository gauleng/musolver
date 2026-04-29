use std::{fmt::Debug, rc::Rc};

use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};

use crate::{
    Cfr, CfrMethod, Game,
    solver::{GameConfig, GameType, LanceGame, MusGame, MusGameTwoHands, MusGameTwoPlayers},
};

pub struct Trainer {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrainerConfig {
    pub method: CfrMethod,
    pub iterations: usize,
}

impl Trainer {
    pub fn train(&self, game_config: &GameConfig, trainer_config: &TrainerConfig) -> Cfr {
        let mut cfr = Cfr::new();
        let mut utility_table = MusGameTwoPlayers::default_utility_table();
        (35..40).rev().for_each(|t1| {
            (0..(40 - t1)).for_each(|t2| {
                let tantos = [t1 + t2, 39 - t2];
                match game_config.game_type {
                    GameType::LanceGame(lance) => {
                        let mut lance_game =
                            LanceGame::new(lance, tantos, game_config.abstract_game);
                        train_game(&mut cfr, &mut lance_game, trainer_config);
                    }
                    GameType::MusGame => {
                        let mut mus_game = MusGame::new(tantos, game_config.abstract_game);
                        train_game(&mut cfr, &mut mus_game, trainer_config);
                    }
                    GameType::LanceGameTwoHands(_) => todo!(),
                    GameType::MusGameTwoHands => {
                        let mut mus_game = MusGameTwoHands::new(tantos, game_config.abstract_game);
                        train_game(&mut cfr, &mut mus_game, trainer_config);
                    }
                    GameType::MusGameTwoPlayers => {
                        let mut mus_game = MusGameTwoPlayers::new(
                            tantos,
                            game_config.abstract_game,
                            Rc::new(utility_table),
                        );
                        train_game(&mut cfr, &mut mus_game, trainer_config);
                        let expected_utility = cfr.expected_utility(&mus_game)[0];
                        println!("Finished training.");
                        println!(
                            "Expected utility {}-{}: {}",
                            tantos[0], tantos[1], expected_utility
                        );
                        println!();
                        utility_table[tantos[0] as usize][tantos[1] as usize] = expected_utility;
                    }
                }
            });
        });
        cfr
    }
}

fn train_game<G>(cfr: &mut Cfr, game: &mut G, trainer_config: &TrainerConfig)
where
    G: Game + Debug + Clone,
    G::Action: Eq + Copy,
{
    use std::time::Instant;

    let now = Instant::now();
    let pb = ProgressBar::new(trainer_config.iterations as u64);
    pb.set_style(
        ProgressStyle::with_template("{wide_bar:40.cyan/blue} {human_pos}/{human_len} {msg} ")
            .unwrap()
            .progress_chars("##-"),
    );
    cfr.train(
        game,
        trainer_config.method,
        trainer_config.iterations,
        |i, util| {
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
        },
    );
    let elapsed = now.elapsed();
    println!("Elapsed: {elapsed:.2?}");
}
