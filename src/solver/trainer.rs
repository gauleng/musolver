use std::{fmt::Debug, rc::Rc};

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
                let cfr = &mut cfrs[tantos[0] as usize][tantos[1] as usize];
                let lance_game = LanceGame::new(lance, tantos, abstract_game);
                train_game(cfr, &lance_game, trainer_config);
                let expected_utility = cfr.expected_utility(&lance_game)[0];
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
                let cfr = &mut cfrs[tantos[0] as usize][tantos[1] as usize];
                let mus_game = MusGame::new(tantos, abstract_game, max_mus_rounds)
                    .with_utility_table(Rc::new(utility_table));
                let expected_utility_players = train_game(cfr, &mus_game, trainer_config);
                let expected_utility = (expected_utility_players[0] + expected_utility_players[2]
                    - expected_utility_players[1]
                    - expected_utility_players[3])
                    / 4.;
                utility_table[tantos[0] as usize][tantos[1] as usize] = expected_utility;
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
                let cfr = &mut cfrs[tantos[0] as usize][tantos[1] as usize];
                let mus_game = MusGameTwoHands::new(tantos, abstract_game, max_mus_rounds)
                    .with_utility_table(Rc::new(utility_table));
                let expected_utility_players = train_game(cfr, &mus_game, trainer_config);
                let expected_utility =
                    (expected_utility_players[0] - expected_utility_players[1]) / 2.;
                utility_table[tantos[0] as usize][tantos[1] as usize] = expected_utility;
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
                let cfr = &mut cfrs[tantos[0] as usize][tantos[1] as usize];
                let mus_game = MusGameTwoPlayers::new(tantos, abstract_game, max_mus_rounds)
                    .with_utility_table(Rc::new(utility_table));
                let expected_utility_players = train_game(cfr, &mus_game, trainer_config);
                let expected_utility =
                    (expected_utility_players[0] - expected_utility_players[1]) / 2.;
                utility_table[tantos[0] as usize][tantos[1] as usize] = expected_utility;
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

fn train_game<G>(cfr: &mut Cfr<G>, game: &G, trainer_config: &TrainerConfig) -> Vec<f64>
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
    let mut last_util = vec![0.; G::N_PLAYERS];
    cfr.train(
        game,
        trainer_config.method,
        trainer_config.iterations,
        |i, util| {
            if i.is_multiple_of(1000) {
                pb.set_position(*i as u64);
                pb.set_message(format!(
                    "Utility: {}",
                    util.iter()
                        .map(|u| format!("{u:.5}"))
                        .collect::<Vec<String>>()
                        .join(" "),
                ));
                last_util.copy_from_slice(util);
            }
        },
    );
    let elapsed = now.elapsed();
    println!("Elapsed: {elapsed:.2?}");
    last_util
}
