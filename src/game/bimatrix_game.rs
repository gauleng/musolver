use ndarray::prelude::*;
use ndarray::Zip;

struct Tableau {
    basis: Vec<isize>,
    coefficients: Array2<f32>,
}

struct Tableaux(Tableau, Tableau);

impl Tableaux {
    fn new(game: &BimatrixGame) -> Tableaux {
        let num_strategies = game.payoff.0.shape();
        let num_labels = game.total_strategies();
        let mut tableaux = Tableaux(
            Tableau {
                basis: (-(num_strategies[0] as isize)..=-1).rev().collect(),
                coefficients: Array2::<f32>::zeros((num_strategies[0], num_labels + 1)),
            },
            Tableau {
                basis: (-(num_labels as isize)..-(num_strategies[0] as isize))
                    .rev()
                    .collect(),
                coefficients: Array2::<f32>::zeros((num_strategies[1], num_labels + 1)),
            },
        );

        tableaux.0.coefficients.column_mut(0).fill(1.);
        tableaux.1.coefficients.column_mut(0).fill(1.);

        Zip::from(&mut tableaux.0.coefficients.slice_mut(s![
            0..num_strategies[0],
            num_strategies[0] + 1..num_labels + 1
        ]))
        .and(&game.payoff.0)
        .for_each(|a, b| *a = -b);

        Zip::from(
            &mut tableaux
                .1
                .coefficients
                .slice_mut(s![0..num_strategies[1], 1..num_strategies[0] + 1]),
        )
        .and(&game.payoff.1.t())
        .for_each(|a, b| *a = -b);

        return tableaux;
    }

    fn get_tableau<'a>(&'a mut self, var: isize) -> &'a mut Tableau {
        let rows0 = self.0.basis.len() as isize;
        let rows1 = self.1.basis.len() as isize;
        let total = rows0 + rows1;

        if (0 < var && var <= rows0) || (-total <= var && var < -rows0) {
            return &mut self.1;
        }
        return &mut self.0;
    }
}

pub struct BimatrixGame {
    payoff: (Array2<f32>, Array2<f32>),
}

#[derive(Debug)]
pub struct Strategy(Array1<f32>, Array1<f32>);

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

    fn pivot(&self, tableaux: &mut Tableaux, e_var: isize) -> isize {
        let t = tableaux.get_tableau(e_var);
        let e_var_col = e_var.abs() as usize;

        let (Some(min_row), _) = Zip::indexed(&t.coefficients.column(0))
            .and(&t.coefficients.column(e_var_col))
            .fold((None, f32::MAX), |(min_row, min_ratio), i, a, b| {
                if *b < 0. {
                    let ratio = -*a / *b;
                    if ratio < min_ratio {
                        (Some(i), ratio)
                    } else {
                        (min_row, min_ratio)
                    }
                } else {
                    (min_row, min_ratio)
                }
            })
        else {
            todo!();
        };

        let l_var = t.basis[min_row];
        t.basis[min_row] = e_var;
        let l_var_col = l_var.abs() as usize;
        let e_var_coeff = -t.coefficients[[min_row, e_var_col]];
        t.coefficients[[min_row, l_var_col]] = -1.;
        t.coefficients[[min_row, e_var_col]] = 0.;

        t.coefficients
            .row_mut(min_row)
            .iter_mut()
            .for_each(|b| *b = *b / e_var_coeff);

        let r = t.coefficients.row(min_row).into_owned();
        for mut row in t.coefficients.rows_mut() {
            let e_var_coeff = row[e_var_col];
            row.scaled_add(e_var_coeff, &r);
            row[e_var_col] = 0.;
        }

        return l_var;
    }

    pub fn lemke_howson(&self) -> Strategy {
        let mut t = Tableaux::new(self);
        let init_e_var = 1;
        let mut l_var = self.pivot(&mut t, init_e_var);
        loop {
            l_var = self.pivot(&mut t, -l_var);
            if l_var.abs() == init_e_var {
                break;
            }
        }
        let mut sorted = Array::zeros(self.total_strategies());
        for (i, v) in t.0.basis.iter().enumerate() {
            let mut prob;
            if *v < 0 {
                prob = 0.;
            } else {
                prob = t.0.coefficients[[i, 0]];
            }
            if prob < 0. || prob > 1. {
                prob = 0.;
            }
            let variable = ((*v).abs() - 1) as usize;
            sorted[[variable]] = prob;
        }
        for (i, v) in t.1.basis.iter().enumerate() {
            let mut prob;
            if *v < 0 {
                prob = 0.;
            } else {
                prob = t.1.coefficients[[i, 0]];
            }
            if prob < 0. || prob > 1. {
                prob = 0.;
            }
            let variable = ((*v).abs() - 1) as usize;
            sorted[[variable]] = prob;
        }
        let eq1 = sorted.slice(s![0..self.num_strategies_player(0)]);
        let eq2 = sorted.slice(s![self.num_strategies_player(0)..self.total_strategies()]);
        Strategy(&eq1 / eq1.sum(), &eq2 / eq2.sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_abs_diff_eq;

    fn test_tableaux() -> Tableaux {
        let a = array![[1., 2., 3.], [4., 5., 6.],];
        let b = array![[7., 8., 9.], [10., 11., 12.],];
        let game = BimatrixGame::new(a, b);
        let tableaux = Tableaux::new(&game);

        tableaux
    }

    #[test]
    #[should_panic]
    fn bimatrixgame_new() {
        let a = array![[2., 1.], [1., 2.]];
        let b = array![[2., 1.], [1., 2.], [3., 4.]];

        let _game = BimatrixGame::new(a, b);
    }

    #[test]
    fn tableau_size() {
        let tableaux = test_tableaux();

        assert_eq!(tableaux.0.basis.len(), 2);
        assert_eq!(tableaux.1.basis.len(), 3);
        assert_eq!(tableaux.0.coefficients.shape(), [2, 6]);
        assert_eq!(tableaux.1.coefficients.shape(), [3, 6]);
    }

    #[test]
    fn tableau_content() {
        let tableaux = test_tableaux();

        assert_eq!(tableaux.0.basis, vec![-1, -2]);
        assert_eq!(
            tableaux.0.coefficients,
            array![[1., 0., 0., -1., -2., -3.], [1., 0., 0., -4., -5., -6.]]
        );

        assert_eq!(tableaux.1.basis, vec![-3, -4, -5]);
        assert_eq!(
            tableaux.1.coefficients,
            array![
                [1., -7., -10., 0., 0., 0.],
                [1., -8., -11., 0., 0., 0.],
                [1., -9., -12., 0., 0., 0.]
            ]
        );
    }

    #[test]
    fn tableau_for_var() {
        let mut tableaux = test_tableaux();

        let t = tableaux.get_tableau(1);
        assert_eq!(t.basis.len(), 3);
        let t = tableaux.get_tableau(2);
        assert_eq!(t.basis.len(), 3);
        let t = tableaux.get_tableau(3);
        assert_eq!(t.basis.len(), 2);
        let t = tableaux.get_tableau(4);
        assert_eq!(t.basis.len(), 2);
        let t = tableaux.get_tableau(5);
        assert_eq!(t.basis.len(), 2);
        let t = tableaux.get_tableau(-1);
        assert_eq!(t.basis.len(), 2);
        let t = tableaux.get_tableau(-2);
        assert_eq!(t.basis.len(), 2);
        let t = tableaux.get_tableau(-3);
        assert_eq!(t.basis.len(), 3);
        let t = tableaux.get_tableau(-4);
        assert_eq!(t.basis.len(), 3);
        let t = tableaux.get_tableau(-5);
        assert_eq!(t.basis.len(), 3);
    }

    #[test]
    fn tableau_pivot() {
        let a = array![[1., 3., 0.], [0., 0., 2.], [2., 1., 1.]];
        let b = array![[2., 1., 0.], [1., 3., 1.], [0., 0., 3.]];
        let game = BimatrixGame::new(a, b);
        let mut tableaux = Tableaux::new(&game);
        let l_var = game.pivot(&mut tableaux, 1);

        assert_eq!(l_var, -4);
        assert_abs_diff_eq!(
            tableaux.1.coefficients,
            arr2(&[
                [0.5, 0., -0.5, 0., -0.5, 0., 0.],
                [0.5, 0.0, -2.5, 0., 0.5, 0., 0.],
                [1., 0., -1., -3., 0., 0., 0.]
            ]),
            epsilon = 1e-5,
        );
    }

    #[test]
    fn lemke_howson() {
        let a = array![[1., 3., 0.], [0., 0., 2.], [2., 1., 1.]];
        let b = array![[2., 1., 0.], [1., 3., 1.], [0., 0., 3.]];
        let game = BimatrixGame::new(a, b);

        let s = game.lemke_howson();

        assert_eq!(
            s,
            Strategy(
                array![6. / 13., 3. / 13., 4. / 13.],
                array![1. / 9., 1. / 3., 5. / 9.]
            )
        );
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
