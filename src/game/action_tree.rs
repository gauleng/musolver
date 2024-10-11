use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

/// Represents an action node in an extensive form game.
///
/// There are two types of nodes, Termimal
/// nodes with no further actions, and NonTerminal nodes with an identifier of the player P to
/// play, and a list of possible actions A.
///
/// # Example
///
/// Let us consider a game for two players identified by integers 0 and 1 and three possible actions,
/// Pass, Call and Bet. The first player to act is player 0. A possible action tree for this game is:
///
///     use musolver::ActionNode;
///
///     #[derive(PartialEq, Eq, Clone, Copy)]
///     enum Action {
///         Pass,
///         Call,
///         Bet,
///     }
///
///     let mut root: ActionNode<u8, Action> = ActionNode::new(0);
///
///     let mut pass = root.add_non_terminal_action(Action::Pass, 1).unwrap();
///     let mut pass_bet = pass.add_non_terminal_action(Action::Bet, 0).unwrap();
///     pass_bet.add_terminal_action(Action::Pass);
///     pass_bet.add_terminal_action(Action::Call);
///     pass.add_terminal_action(Action::Pass);
///
///     let mut bet = root.add_non_terminal_action(Action::Bet, 1).unwrap();
///     bet.add_terminal_action(Action::Pass);
///     bet.add_terminal_action(Action::Call);
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionNode<P, A> {
    #[default]
    Terminal,
    NonTerminal(P, Vec<(A, ActionNode<P, A>)>),
}

impl<P, A> ActionNode<P, A>
where
    P: Copy,
    A: Eq + Copy,
{
    /// Create a new non-terminal action node for player P.
    pub fn new(p: P) -> Self {
        Self::NonTerminal(p, Vec::new())
    }

    pub fn add_terminal_action(&mut self, a: A) {
        if let ActionNode::NonTerminal(_, m) = self {
            m.push((a, ActionNode::Terminal));
        }
    }

    /// Adds a non terminal action to the node. It receives the action A that can be played at this
    /// node, and the player P that should act after this action has taken place.
    ///
    /// If this node is terminal the action is not added
    /// and None is returned. Otherwise, it returns a mutable reference to the created node.
    pub fn add_non_terminal_action(&mut self, a: A, p: P) -> Option<&mut Self> {
        if let ActionNode::NonTerminal(_, m) = self {
            let child = Self::new(p);
            m.push((a, child));
            m.last_mut().map(|v| &mut v.1)
        } else {
            None
        }
    }

    pub fn next_node(&self, action: A) -> Option<&ActionNode<P, A>> {
        match self {
            ActionNode::Terminal => None,
            ActionNode::NonTerminal(_, children) => {
                children.iter().find(|&c| c.0 == action).map(|c| &c.1)
            }
        }
    }

    pub fn search_action_node(&self, history: &[A]) -> &ActionNode<P, A> {
        let mut current_node = self;
        history.iter().for_each(|a| {
            current_node = match current_node.next_node(*a) {
                Some(n) => n,
                None => current_node,
            };
        });
        current_node
    }

    pub fn children(&self) -> Option<&Vec<(A, ActionNode<P, A>)>> {
        match self {
            ActionNode::Terminal => None,
            ActionNode::NonTerminal(_, vec) => Some(vec),
        }
    }

    pub fn actions(&self) -> Option<Vec<A>> {
        match self {
            ActionNode::Terminal => None,
            ActionNode::NonTerminal(_, vec) => {
                Some(vec.iter().map(|(action, _)| *action).collect())
            }
        }
    }

    pub fn to_play(&self) -> Option<P> {
        match self {
            ActionNode::Terminal => None,
            ActionNode::NonTerminal(player, _) => Some(*player),
        }
    }

    pub fn from_file(path: &Path) -> std::io::Result<Self>
    where
        A: for<'a> Deserialize<'a>,
        P: for<'a> Deserialize<'a>,
    {
        let contents = fs::read_to_string(path)?;
        let n: ActionNode<P, A> = serde_json::from_str(&contents).unwrap();

        Ok(n)
    }
}
