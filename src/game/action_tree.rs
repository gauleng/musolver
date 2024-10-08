use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionNode<P, A> {
    #[default]
    Terminal,
    NonTerminal(P, Vec<(A, ActionNode<P, A>)>),
}

impl<P, A> ActionNode<P, A>
where
    P: for<'a> Deserialize<'a>,
    A: Eq + Copy + for<'a> Deserialize<'a>,
{
    pub fn new(p: P) -> Self {
        Self::NonTerminal(p, Vec::new())
    }

    pub fn add_terminal_action(&mut self, a: A) {
        if let ActionNode::NonTerminal(_, m) = self {
            m.push((a, ActionNode::Terminal));
        }
    }

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

    pub fn from_file(path: &Path) -> std::io::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let n: ActionNode<P, A> = serde_json::from_str(&contents).unwrap();

        Ok(n)
    }
}
