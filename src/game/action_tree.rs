#[derive(Debug)]
pub enum ActionNode2<P, A> {
    Terminal,
    NonTerminal(P, Vec<(A, ActionNode2<P, A>)>),
}

impl<P, A> ActionNode2<P, A> {
    pub fn new(p: P) -> Self {
        Self::NonTerminal(p, Vec::new())
    }

    pub fn add_terminal_action(&mut self, a: A) {
        if let ActionNode2::NonTerminal(_, m) = self {
            m.push((a, ActionNode2::Terminal));
        }
    }

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
