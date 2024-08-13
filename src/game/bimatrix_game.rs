use ndarray::prelude::*;

pub struct BimatrixGame {
    payoff: (Array2<f32>, Array2<f32>),
}

#[derive(Debug)]
pub struct Strategy(pub Array1<f32>, pub Array1<f32>);

impl PartialEq for Strategy {
    fn eq(&self, other: &Strategy) -> bool {
        self.0.abs_diff_eq(&other.0, 1e-5) && self.1.abs_diff_eq(&other.1, 1e-5)
    }
}

impl BimatrixGame {
    pub fn new(p1: Array2<f32>, p2: Array2<f32>) -> Self {
        if p1.shape() != p2.shape() {
            panic!("Payoff matrices must have same size.");
        }
        BimatrixGame { payoff: (p1, p2) }
    }

    pub fn num_strategies(&self) -> (usize, usize) {
        (self.payoff.0.shape()[0], self.payoff.0.shape()[1])
    }

    pub fn num_strategies_player(&self, player: usize) -> usize {
        self.payoff.0.shape()[player]
    }

    pub fn total_strategies(&self) -> usize {
        self.payoff.0.shape()[0] + self.payoff.1.shape()[1]
    }

    pub fn strategy_payoff(&self, strategy: &Strategy) -> (f32, f32) {
        let payoff0 = self.payoff.0.dot(&strategy.1).dot(&strategy.0);
        let payoff1 = self.payoff.1.dot(&strategy.1).dot(&strategy.0);

        return (payoff0, payoff1);
    }

    pub fn payoff_matrix(&self, player: usize) -> &Array2<f32> {
        if player == 0 {
            &self.payoff.0
        } else {
            &self.payoff.1
        }
    }

    pub fn regret(&self, strategy: &Strategy) -> (f32, f32) {
        let strategy_payoff = self.strategy_payoff(strategy);
        let payoff0 = self.payoff.0.dot(&strategy.1);
        let payoff1 = strategy.0.dot(&self.payoff.1);

        let max_payoff0 = payoff0.into_iter().reduce(f32::max).unwrap();
        let max_payoff1 = payoff1.into_iter().reduce(f32::max).unwrap();
        (
            max_payoff0 - strategy_payoff.0,
            max_payoff1 - strategy_payoff.1,
        )
    }

    pub fn is_ne(&self, strategy: &Strategy) -> bool {
        let r = self.regret(strategy);

        r.0 < 1e-5 && r.1 < 1e-5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn bimatrixgame_new() {
        let a = array![[2., 1.], [1., 2.]];
        let b = array![[2., 1.], [1., 2.], [3., 4.]];

        let _game = BimatrixGame::new(a, b);
    }

    #[test]
    fn regret() {
        let a = array![[1. / 3., 0.], [1., 1.]];
        let b = array![[1. / 3., 1.], [0., 1.]];
        let game = BimatrixGame::new(a, b);

        let s = Strategy(array![0., 1.], array![0., 1.]);

        assert!(game.is_ne(&s));

        let r = Strategy(array![0., 1.], array![0.5, 0.5]);
        assert_eq!(game.regret(&r), (0., 0.5));
    }
}
