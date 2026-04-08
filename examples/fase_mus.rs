use std::array;

use musolver::{Cfr, Game};
use rand::Rng;

fn main() {
    let mut fase_mus = FaseMus::new(10);
    let mut cfr = Cfr::new();

    cfr.train(
        &mut fase_mus,
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
}

impl Game for FaseMus {
    type Action = MusAction;
    const N_PLAYERS: usize = 4;

    fn act(&mut self, a: Self::Action) {
        self.history.push(format!("{:?}", a));
        if a == MusAction::Cortar {
            self.fase = Fase::CompararCartas;
            self.turn = None;
        } else {
            self.turn = match self.turn {
                Some(3) => None,
                Some(i) => Some(i + 1),
                _ => None,
            };
        }
    }

    fn utility(&mut self, player: usize) -> f64 {
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

    fn actions(&self) -> Vec<Self::Action> {
        vec![MusAction::Mus, MusAction::Cortar]
    }

    fn info_set_str(&self, player: usize) -> String {
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
            |turn| musolver::NodeType::Player(turn),
        )
    }

    fn history_str(&self) -> String {
        self.history
            .iter()
            .map(|action| format!("{action:?}"))
            .collect::<Vec<String>>()
            .join(",")
    }

    fn new_random(&mut self) {
        match self.fase {
            Fase::Repartir => {
                self.turn = Some(0);
                self.fase = Fase::Mus;
            }
            Fase::Mus => {
                self.fase = Fase::CompararCartas;
            }
            Fase::CompararCartas => {
                panic!("Llamada a new_random en estado terminal CompararCartas.");
            }
        }
        self.history.push("R".into());
        self.manos = Some(Self::repartir_cartas(self.num_cartas));
    }

    fn reset(&mut self) {
        self.history.clear();
        self.turn = None;
        self.fase = Fase::Repartir;
        self.manos = None;
    }

    fn new_iter<F>(&mut self, _f: F)
    where
        F: FnMut(&mut Self, f64),
    {
        todo!()
    }
}
