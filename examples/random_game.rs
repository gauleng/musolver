use musolver::{BimatrixGame, lemke_howson};
use ndarray::Array;
use ndarray_rand::RandomExt;
use ndarray_rand::rand_distr::Uniform;

fn help() {
    println!("Use: random_game <num strategies player 1> <num strategies player 2>");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.len() {
        1 | 2 => help(),
        3 => {
            let num_strategies0: usize = match args[1].parse() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("First argument is not an integer.");
                    help();
                    return;
                }
            };
            let num_strategies1: usize = match args[2].parse() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("Second argument is not an integer.");
                    help();
                    return;
                }
            };

            let a = Array::random((num_strategies0, num_strategies1), Uniform::new(0., 10.));
            let b = Array::random((num_strategies0, num_strategies1), Uniform::new(0., 10.));

            println!("Payoff for player 1:");
            println!("{a}");
            println!("Payoff for player 2:");
            println!("{b}");

            let game = BimatrixGame::new(a, b);
            let eq = lemke_howson(&game);

            println!("Equilibrium strategies");
            println!("Player 1: {}", eq.0);
            println!("Player 2: {}", eq.1);

            let eq_payoff = game.strategy_payoff(&eq);
            println!("Equilibrium payoff");
            println!("Player 1: {}, Player 2: {}", eq_payoff.0, eq_payoff.1);
        }
        _ => help(),
    }
}
