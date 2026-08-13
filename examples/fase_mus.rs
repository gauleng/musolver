use std::array;

use musolver::{Cfr, Game};
use rand::Rng;

fn main() {
    let fase_mus = FaseMus::new(10);
    let mut cfr = Cfr::new();

    cfr.train(
        &fase_mus,
        musolver::CfrMethod::FsiCfr,
        10000000,
        |_player, _utility| {},
    );

    cfr.nodes()
        .iter()
        .filter(|(_, node)| node.get_average_strategy()[0] > 0.5)
        .for_each(|(info_set, node)| {
            println!(
                "Info set: {info_set}, strategy: {:?}",
                node.get_average_strategy()
            );
        });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MusAction {
    Mus,
    Cortar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Fase {
    Repartir,
    Mus,
    CompararCartas,
}

#[derive(Debug, Clone)]
struct FaseMus {
    history: Vec<String>,
    turn: Option<usize>,
    fase: Fase,
    manos: Option<[usize; 4]>,
    num_cartas: usize,
}

impl FaseMus {
    fn new(num_cartas: usize) -> FaseMus {
        FaseMus {
            history: vec![],
            turn: None,
            fase: Fase::Repartir,
            manos: None,
            num_cartas,
        }
    }

    fn repartir_cartas(num_cartas: usize) -> [usize; 4] {
        let mut rng = rand::thread_rng();
        array::from_fn(|_| rng.gen_range(0..num_cartas))
    }

    fn actions(&self) -> Vec<MusAction> {
        vec![MusAction::Mus, MusAction::Cortar]
    }
}

impl Game for FaseMus {
    type InfoSet = String;
    const N_PLAYERS: usize = 4;

    fn act(&self, action_idx: usize) -> Self {
        let a = self.actions()[action_idx];
        let mut new_game = self.clone();
        new_game.history.push(format!("{:?}", a));
        if a == MusAction::Cortar {
            new_game.fase = Fase::CompararCartas;
            new_game.turn = None;
        } else {
            new_game.turn = match new_game.turn {
                Some(3) => None,
                Some(i) => Some(i + 1),
                _ => None,
            };
        }
        new_game
    }

    fn utility(&self, player: usize) -> f64 {
        assert!(
            self.fase == Fase::CompararCartas,
            "Utility can only be calculated in the CompararCartas phase"
        );
        let manos = self
            .manos
            .expect("Tiene que haber manos para calcular el resultado del juego");
        let mejor_mano = manos[0].max(manos[2]);
        let mejor_postre = manos[1].max(manos[3]);
        if player == 0 || player == 2 {
            if mejor_mano >= mejor_postre { 1. } else { -1. }
        } else {
            if mejor_mano >= mejor_postre { -1. } else { 1. }
        }
    }

    fn info_set(&self, player: usize) -> String {
        self.manos
            .expect("Tiene que haber manos para obtener el info_set_str")[player]
            .to_string()
            + &self.history_str()
    }

    fn current_player(&self) -> musolver::NodeType {
        self.turn.map_or_else(
            || match self.fase {
                Fase::CompararCartas => musolver::NodeType::Terminal,
                Fase::Mus | Fase::Repartir => musolver::NodeType::Chance,
            },
            |turn| musolver::NodeType::Player(turn, self.actions().len()),
        )
    }

    fn history_str(&self) -> String {
        self.history
            .iter()
            .map(|action| format!("{action:?}"))
            .collect::<Vec<String>>()
            .join(",")
    }

    fn chance_sample(&self) -> Self {
        let mut new_game = self.clone();
        match new_game.fase {
            Fase::Repartir => {
                new_game.turn = Some(0);
                new_game.fase = Fase::Mus;
            }
            Fase::Mus => {
                new_game.fase = Fase::CompararCartas;
            }
            Fase::CompararCartas => {
                panic!("Llamada a chance_sample en estado terminal CompararCartas.");
            }
        }
        new_game.history.push("R".into());
        new_game.manos = Some(Self::repartir_cartas(new_game.num_cartas));
        new_game
    }

    fn chance_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        std::iter::empty()
    }
}
