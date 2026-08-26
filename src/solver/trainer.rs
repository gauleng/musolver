use std::{fmt::Debug, sync::Arc};

use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    Cfr, CfrMethod, Game,
    mus::Lance,
    solver::{LanceGame, MusGame, MusGameTwoHands, MusGameTwoPlayers},
};

pub struct Trainer {
    tantos: [u8; 2],
}

#[derive(
    Debug,
    serde::Serialize,
    serde::Deserialize,
    rkyv::Serialize,
    rkyv::Deserialize,
    rkyv::Archive,
    Clone,
)]
pub struct TrainerConfig {
    pub method: CfrMethod,
    pub iterations: usize,
}

impl Trainer {
    pub fn new() -> Self {
        Self { tantos: [0; 2] }
    }

    pub fn with_tantos(self, tantos: [u8; 2]) -> Self {
        Self { tantos }
    }

    pub fn train_lance_game(
        &self,
        lance: Lance,
        abstract_game: bool,
        trainer_config: &TrainerConfig,
    ) -> [[Cfr<LanceGame>; 40]; 40] {
        let mut cfrs = std::array::from_fn(|_| std::array::from_fn(|_idx| Cfr::new()));
        let target = self.tantos;
        (0..40).rev().for_each(|t1| {
            for t2 in 0..(40 - t1) {
                let tantos = [t1 + t2, 39 - t2];
                if tantos[0] < target[0] || tantos[1] < target[1] {
                    continue;
                }
                let lance_game = LanceGame::new(lance, tantos, abstract_game);
                let cfr = train_game(&lance_game, trainer_config);
                let expected_utility = cfr.utility()[0];
                cfrs[tantos[0] as usize][tantos[1] as usize] = cfr;
                println!("Finished training.");
                println!(
                    "Expected utility {}-{}: {}",
                    tantos[0], tantos[1], expected_utility
                );
                println!();
            }
        });
        cfrs
    }

    pub fn train_mus_game(
        &self,
        abstract_game: bool,
        max_mus_rounds: u8,
        trainer_config: &TrainerConfig,
    ) -> [[Cfr<MusGame>; 40]; 40] {
        let mut cfrs = std::array::from_fn(|_| std::array::from_fn(|_idx| Cfr::new()));
        let mut utility_table = MusGame::default_utility_table();
        let target = self.tantos;
        (0..40).rev().for_each(|t1| {
            for t2 in 0..(40 - t1) {
                let tantos = [t1 + t2, 39 - t2];
                if tantos[0] < target[0] || tantos[1] < target[1] {
                    continue;
                }
                let mus_game = MusGame::new(tantos, abstract_game, max_mus_rounds)
                    .with_utility_table(Arc::new(utility_table));
                let cfr = train_game(&mus_game, trainer_config);
                let expected_utility_players = cfr.utility();
                let expected_utility = (expected_utility_players[0] + expected_utility_players[2]
                    - expected_utility_players[1]
                    - expected_utility_players[3])
                    / 4.;
                utility_table[tantos[0] as usize][tantos[1] as usize] = expected_utility;
                cfrs[tantos[0] as usize][tantos[1] as usize] = cfr;
                println!("Finished training.");
                println!(
                    "Expected utility {}-{}: {}",
                    tantos[0], tantos[1], expected_utility
                );
                println!();
            }
        });
        cfrs
    }

    pub fn train_mus_game_two_hands(
        &self,
        abstract_game: bool,
        max_mus_rounds: u8,
        trainer_config: &TrainerConfig,
    ) -> [[Cfr<MusGameTwoHands>; 40]; 40] {
        let mut cfrs = std::array::from_fn(|_| std::array::from_fn(|_idx| Cfr::new()));
        let mut utility_table = MusGame::default_utility_table();
        let target = self.tantos;
        (0..40).rev().for_each(|t1| {
            for t2 in 0..(40 - t1) {
                let tantos = [t1 + t2, 39 - t2];
                if tantos[0] < target[0] || tantos[1] < target[1] {
                    continue;
                }
                let mus_game = MusGameTwoHands::new(tantos, abstract_game, max_mus_rounds)
                    .with_utility_table(Arc::new(utility_table));
                let cfr = train_game(&mus_game, trainer_config);
                let expected_utility_players = cfr.utility();
                let expected_utility =
                    (expected_utility_players[0] - expected_utility_players[1]) / 2.;
                utility_table[tantos[0] as usize][tantos[1] as usize] = expected_utility;
                cfrs[tantos[0] as usize][tantos[1] as usize] = cfr;
                println!("Finished training.");
                println!(
                    "Expected utility {}-{}: {}",
                    tantos[0], tantos[1], expected_utility
                );
                println!();
            }
        });
        cfrs
    }

    pub fn train_mus_game_two_players(
        &self,
        abstract_game: bool,
        max_mus_rounds: u8,
        trainer_config: &TrainerConfig,
    ) -> [[Cfr<MusGameTwoPlayers>; 40]; 40] {
        let mut cfrs = std::array::from_fn(|_| std::array::from_fn(|_idx| Cfr::new()));
        let mut utility_table = MusGame::default_utility_table();
        let target = self.tantos;
        (0..40).rev().for_each(|t1| {
            for t2 in 0..(40 - t1) {
                let tantos = [t1 + t2, 39 - t2];
                if tantos[0] < target[0] || tantos[1] < target[1] {
                    continue;
                }
                let mus_game = MusGameTwoPlayers::new(tantos, abstract_game, max_mus_rounds)
                    .with_utility_table(Arc::new(utility_table));
                let cfr = train_game(&mus_game, trainer_config);
                let expected_utility_players = cfr.utility();
                let expected_utility =
                    (expected_utility_players[0] - expected_utility_players[1]) / 2.;
                utility_table[tantos[0] as usize][tantos[1] as usize] = expected_utility;
                cfrs[tantos[0] as usize][tantos[1] as usize] = cfr;
                println!("Finished training.");
                println!(
                    "Expected utility {}-{}: {}",
                    tantos[0], tantos[1], expected_utility
                );
                println!();
            }
        });
        cfrs
    }
}

impl Default for Trainer {
    fn default() -> Self {
        Self::new()
    }
}

fn train_game<G>(game: &G, trainer_config: &TrainerConfig) -> Cfr<G>
where
    G: Game + Debug + Clone,
{
    use std::time::Instant;

    let now = Instant::now();
    let pb = ProgressBar::new(trainer_config.iterations as u64);
    pb.set_style(
        ProgressStyle::with_template("{wide_bar:40.cyan/blue} {human_pos}/{human_len} {msg} ")
            .unwrap()
            .progress_chars("##-"),
    );
    let mut cfr = Cfr::new()
        .method(trainer_config.method)
        .on_progress(move |i, util| {
            if i.is_multiple_of(1000) {
                pb.set_position(i as u64);
                pb.set_message(format!(
                    "Utility: {}",
                    util.iter()
                        .map(|u| format!("{u:.5}"))
                        .collect::<Vec<String>>()
                        .join(" "),
                ));
            }
        });
    cfr.train(game, trainer_config.iterations);
    let elapsed = now.elapsed();
    println!("Elapsed: {elapsed:.2?}");
    cfr
}
