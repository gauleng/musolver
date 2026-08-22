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
pub trait Game: Sized {
    /// Type used as a key to identify information sets.
    type InfoSet: Eq + Hash;

    /// Number of players of the game.
    const N_PLAYERS: usize;

    /// Utility function for the given player in a terminal node.
    fn utility(&self, player: usize) -> f64;

    /// Key identifying the information set for the current player.
    fn info_set(&self, player: usize) -> Self::InfoSet;

    // String representation of the history leading to the current state.
    fn history_str(&self) -> String;

    // Returns if the current node is a chance, terminal or player node.
    fn current_node(&self) -> NodeType;

    /// Advance the state with the given action index for the current player.
    fn act(&self, action_idx: usize) -> Self;

    // Picks a random action in chance nodes.
    fn chance_sample(&self) -> Self;

    /// Returns an iterator for all available actions in chance nodes.
    fn chance_iter(&self) -> impl Iterator<Item = (Self, f64)>;
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

impl Rps {
    fn actions(&self) -> Vec<RpsAction> {
        vec![RpsAction::Rock, RpsAction::Paper, RpsAction::Scissors]
    }
}

impl Game for Rps {
    type InfoSet = usize;
    const N_PLAYERS: usize = 2;

    fn utility(&self, player: usize) -> f64 {
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
        if player == 0 { payoff } else { -payoff }
    }

    fn info_set(&self, player: usize) -> usize {
        player
    }

    fn history_str(&self) -> String {
        self.history
            .iter()
            .map(|action| format!("{action:?}"))
            .collect::<Vec<String>>()
            .join(",")
    }

    fn current_node(&self) -> musolver::NodeType {
        self.turn.map_or_else(
            || musolver::NodeType::Terminal,
            |turn| musolver::NodeType::Player(turn, self.actions().len()),
        )
    }

    fn act(&self, action_idx: usize) -> Self {
        let a = self.actions()[action_idx];
        let mut new_game = self.clone();
        new_game.history.push(a);
        new_game.turn = match new_game.turn {
            Some(0) => Some(1),
            _ => None,
        };
        new_game
    }

    fn chance_sample(&self) -> Self {
        todo!()
    }

    fn chance_iter(&self) -> impl Iterator<Item = (Self, f64)> {
        std::iter::empty()
    }
}
```

Run the example with

```bash
cargo run --example rps
```

Each action should have a probability of about 1/3 for both players.

## Acknowledgments

This project builds upon ideas and code from:

- [rs-poker](https://github.com/elliottneilclark/rs-poker) - A Rust poker library that inspired parts of the card game abstractions. I learned a lot about Rust with it.
- [cpp-cfr](https://github.com/bakanaouji/cpp-cfr) - A C++ implementation of CFR.
