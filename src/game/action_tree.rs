use std::collections::HashMap;
use std::hash::Hash;

#[derive(Debug)]
pub enum ActionNode<P, A> {
    Terminal,
    NonTerminal(P, HashMap<A, ActionNode<P, A>>),
}

pub enum ActionNode2<P, A> {
    Terminal,
    NonTerminal(P, Vec<(A, ActionNode2<P, A>)>),
}

impl<P, A> ActionNode2<P, A> {
    pub fn new(p: P) -> Self {
        Self::NonTerminal(p, Vec::new())
    }

    pub fn add_terminal_action(&mut self, a: A)
    where
        A: Hash + Eq,
    {
        if let ActionNode2::NonTerminal(_, m) = self {
            m.push((a, ActionNode2::Terminal));
        }
    }

    // pub fn iter(&self) -> Iter<P, A> {
    //     if let ActionNode2::NonTerminal(_, m) = self {
    //         Iter::NonTerminal(m.iter())
    //     } else {
    //         Iter::Terminal
    //     }
    // }

    pub fn add_non_terminal_action(&mut self, a: A, p: P) -> Option<&mut Self> {
        if let ActionNode2::NonTerminal(_, m) = self {
            let child = Self::new(p);
            m.push((a, child));
            m.last_mut().map(|v| &mut v.1)
        } else {
            None
        }
    }
}
pub enum Iter<'a, P, A> {
    Terminal,
    NonTerminal(std::collections::hash_map::Iter<'a, A, ActionNode<P, A>>),
}

impl<'a, P, A> Iterator for Iter<'a, P, A> {
    type Item = <std::collections::hash_map::Iter<'a, A, ActionNode<P, A>> as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Terminal => None,
            Self::NonTerminal(i) => i.next(),
        }
    }
}

impl<P, A> ActionNode<P, A> {
    pub fn new(p: P) -> Self {
        Self::NonTerminal(p, HashMap::new())
    }

    pub fn add_terminal_action(&mut self, a: A)
    where
        A: Hash + Eq,
    {
        if let ActionNode::NonTerminal(_, m) = self {
            m.insert(a, ActionNode::Terminal);
        }
    }

    pub fn iter(&self) -> Iter<P, A> {
        if let ActionNode::NonTerminal(_, m) = self {
            Iter::NonTerminal(m.iter())
        } else {
            Iter::Terminal
        }
    }

    pub fn add_non_terminal_action(&mut self, a: A, p: P) -> Option<&mut Self>
    where
        A: Hash + Eq + Copy,
    {
        if let ActionNode::NonTerminal(_, m) = self {
            let child = Self::new(p);
            m.insert(a, child);
            m.get_mut(&a)
        } else {
            None
        }
    }
}
