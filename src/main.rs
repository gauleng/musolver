//use ndarray::prelude::*;
use ndarray::Array;
use ndarray_rand::rand_distr::Uniform;
use ndarray_rand::RandomExt;

pub mod game;
use game::bimatrix_game::*;

pub mod mus;

fn main() {
    let a = Array::random((30, 40), Uniform::new(0., 10.));
    let b = Array::random((30, 40), Uniform::new(0., 10.));
    //let a = array![[3., 5., 6.], [6., 1., 5.],];
    //let b = array![[4., 2., 3.], [2., 4., 1.],];

    let game = BimatrixGame::new(a, b);

    let eq = game::lemke_howson::lemke_howson(&game);

    println!("{:?}", game.strategy_payoff(&eq));
    println!("regret: {:?}", game.regret(&eq));
}
