use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ActionNode<P, A> {
    Terminal,
    NonTerminal(P, Vec<(A, ActionNode<P, A>)>),
}

impl<P, A> ActionNode<P, A>
where
    A: Eq,
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
}
