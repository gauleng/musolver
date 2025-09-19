# musolver

Musolver is a set of tools to generate, inspect and test strategies for the game of Mus.

## Usage

### Solver

The solver uses Counterfactual Regret Minimization (CFR) to find Nash equilibrium strategies. To compute a strategy for the entire game run:

```bash
cargo run --release -- --iter 1000000 --method fsi-cfr
```

Available methods are:
- `cfr` - Vanilla CFR.
- `cfr-plus` - CFR+ with linear averaging.
- `chance-sampling` - CFR with chance sampling. (default method)
- `external-sampling` - CFR with external sampling.
- `fsi-cfr` - Fixed-Strategy Iteration CFR.

Other parameters are:
- `--abstract-game` - Considers  different hand abstractions for each lance.
- `--lance <LANCE>` - Computes a strategy for an isolated lance. For example: `--lance punto`

### Inspector 

The inspector provides a GUI to analyze the computed strategies:

```bash
cargo run --release -p inspector
```

## Game Solving Library

The crate provides a generic framework for solving imperfect information games using CFR variants. The main trait is `Game`:

```rust
pub trait Game {
    type Action;
    const N_PLAYERS: usize;

    // Returns utility/payoff for given player in current state
    fn utility(&mut self, player: usize) -> f64;

    // String representation of information set for given player
    fn info_set_str(&self, player: usize) -> String; 

    // Available actions in current state
    fn actions(&self) -> Vec<Self::Action>;

    // Returns if the current node is a chance, terminal or player node.
    fn current_player(&self) -> NodeType;

    // Advance game state with given action
    fn act(&mut self, a: Self::Action);

    // Picks a random action in chance nodes.
    fn new_random(&mut self);


}
```

### Example: Rock Paper Scissors

See `examples/rps.rs` for a complete implementation of Rock Paper Scissors:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RpsAction {
    Rock,
    Paper,
    Scissors,
}

#[derive(Debug, Clone)]
struct Rps {
    history: Vec<RpsAction>,
    turn: Option<usize>,
}

impl Game for RPS {
    type Action = RpsAction;
    const N_PLAYERS: usize = 2;

    fn utility(&mut self, player: usize) -> f64 {
        let (action1, action2) = (&self.history[0], &self.history[1]);
        let payoff = match (action1, action2) {
            (RpsAction::Rock, RpsAction::Scissors) => 1.,
            (RpsAction::Rock, RpsAction::Paper) => -1.,
            (RpsAction::Paper, RpsAction::Scissors) => -1.,
            (RpsAction::Paper, RpsAction::Rock) => 1.,
            (RpsAction::Scissors, RpsAction::Rock) => -1.,
            (RpsAction::Scissors, RpsAction::Paper) => 1.,
            _ => 0.,
        };
        if player == 0 {
            payoff
        } else {
            -payoff
        }
    }

    fn info_set_str(&self, player: usize) -> String {
        player.to_string()
    }

    fn actions(&self) -> Vec<Self::Action> {
        vec![RpsAction::Rock, RpsAction::Paper, RpsAction::Scissors]
    }

    fn current_player(&self) -> musolver::NodeType {
        self.turn.map_or_else(
            || musolver::NodeType::Terminal,
            |turn| musolver::NodeType::Player(turn),
        )
    }

    fn act(&mut self, a: Self::Action) {
        self.history.push(a);
        self.turn = match self.turn {
            Some(0) => Some(1),
            _ => None,
        };
    }
    
    // ...rest of implementation
}
```

Run the exampe with

```bash
cargo run --example rps
```

Each action should have a probability of about 1/3 for both players.

## Acknowledgments

This project builds upon ideas and code from:

- [rs-poker](https://github.com/elliottneilclark/rs-poker) - A Rust poker library that inspired parts of the card game abstractions. I learned a lot about Rust with it.
- [cpp-cfr](https://github.com/bakanaouji/cpp-cfr) - A C++ implementation of CFR.
