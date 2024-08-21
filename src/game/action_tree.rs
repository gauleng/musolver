use std::collections::HashMap;
use std::hash::Hash;

enum ActionNode<P, A> {
    Terminal,
    NonTerminal(P, HashMap<A, ActionNode<P, A>>),
}

impl<P, A> ActionNode<P, A> {
    fn add_terminal_action(&mut self, a: A)
    where
        A: Hash + Eq,
    {
        match self {
            ActionNode::NonTerminal(_, m) => {
                m.insert(a, ActionNode::Terminal);
            }
            _ => {}
        };
    }
}
