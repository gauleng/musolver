use std::collections::{VecDeque, hash_map::Entry};

use arrayvec::ArrayVec;
use rustc_hash::FxHashMap;

use crate::Game;

use super::NodeType;

#[derive(Debug)]
pub struct GameNode<G: Game, D> {
    game: G,
    next_nodes: ArrayVec<usize, 16>,
    info_set: Option<G::InfoSet>,
    data: D,
}

impl<G: Game, D> GameNode<G, D> {
    pub fn data(&self) -> &D {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut D {
        &mut self.data
    }

    pub fn game_mut(&mut self) -> &mut G {
        &mut self.game
    }

    pub fn game(&self) -> &G {
        &self.game
    }

    pub fn info_set_str(&self) -> Option<&G::InfoSet> {
        self.info_set.as_ref()
    }

    pub fn children(&self) -> &[usize] {
        &self.next_nodes
    }
}

pub struct GameGraph<G: Game, D> {
    game: G,
    node_ids: FxHashMap<u64, usize>,
    game_nodes: Vec<GameNode<G, D>>,
    order: Vec<usize>,
}

impl<G, D> GameGraph<G, D>
where
    G: Game + Clone,
    D: Default,
{
    pub fn new(game: &G) -> Self {
        let mut new_graph = Self {
            game: game.clone(),
            node_ids: FxHashMap::default(),
            game_nodes: Vec::new(),
            order: Vec::new(),
        };
        new_graph.seed_game();
        new_graph
    }

    pub fn reset(&mut self) {
        self.node_ids.clear();
        self.game_nodes.clear();
        self.order.clear();
        self.seed_game();
    }

    fn seed_game(&mut self) {
        let node_key = self.game.node_key();
        let current_node = self.game.current_node();
        let info_set_str = match current_node {
            NodeType::Chance | NodeType::Terminal => None,
            NodeType::Player(player_id, _) => Some(self.game.info_set(player_id)),
        };
        self.node_ids.insert(node_key, 0);
        self.game_nodes.push(GameNode {
            game: self.game.clone(),
            next_nodes: ArrayVec::new(),
            info_set: info_set_str,
            data: D::default(),
        });
    }

    pub fn inflate(&mut self) {
        let mut game_list = VecDeque::from([0]);
        while let Some(parent_idx) = game_list.pop_front() {
            self.next_nodes(parent_idx, &mut game_list);
        }
        self.topological_order();
    }

    pub fn nodes(&self) -> &[GameNode<G, D>] {
        &self.game_nodes
    }

    pub fn nodes_mut(&mut self) -> &mut [GameNode<G, D>] {
        &mut self.game_nodes
    }

    pub fn node(&self, idx: usize) -> &GameNode<G, D> {
        &self.game_nodes[idx]
    }

    pub fn node_mut(&mut self, idx: usize) -> &mut GameNode<G, D> {
        &mut self.game_nodes[idx]
    }

    pub fn num_nodes(&self) -> usize {
        self.game_nodes.len()
    }

    pub fn order(&self) -> &[usize] {
        &self.order
    }

    fn next_nodes(&mut self, idx: usize, new_nodes: &mut VecDeque<usize>) {
        let game = &self.game_nodes[idx].game;
        match game.current_node() {
            NodeType::Chance => {
                let new_game = self.game_nodes[idx].game.chance_sample();

                if let Some(child_idx) = self.append_child(idx, new_game) {
                    new_nodes.push_back(child_idx);
                }
            }
            NodeType::Player(_, num_actions) => {
                new_nodes.extend((0..num_actions).filter_map(|action| {
                    let new_game = self.game_nodes[idx].game.act(action);
                    self.append_child(idx, new_game)
                }))
            }
            NodeType::Terminal => {}
        }
    }

    fn append_child(&mut self, parent_idx: usize, new_game: G) -> Option<usize> {
        let history_str = new_game.node_key();
        match self.node_ids.entry(history_str) {
            Entry::Occupied(next_id) => {
                self.game_nodes[parent_idx].next_nodes.push(*next_id.get());
                None
            }
            Entry::Vacant(vacant_entry) => {
                let current_node = new_game.current_node();
                let info_set_str = match current_node {
                    NodeType::Chance | NodeType::Terminal => None,
                    NodeType::Player(player_id, _) => Some(new_game.info_set(player_id)),
                };
                let last_node_id = self.game_nodes.len();
                vacant_entry.insert(last_node_id);
                self.game_nodes.push(GameNode {
                    game: new_game,
                    next_nodes: ArrayVec::new(),
                    info_set: info_set_str,
                    data: D::default(),
                });
                self.game_nodes[parent_idx].next_nodes.push(last_node_id);
                Some(last_node_id)
            }
        }
    }

    fn topological_order(&mut self) {
        let num_nodes = self.num_nodes();

        let mut num_parents = vec![0usize; num_nodes];

        for node in &self.game_nodes {
            for child in node.children() {
                num_parents[*child] += 1;
            }
        }

        let mut queue: VecDeque<usize> = VecDeque::from([0]);
        while let Some(node) = queue.pop_front() {
            self.order.push(node);
            for &child in self.game_nodes[node].children() {
                num_parents[child] -= 1;
                if num_parents[child] == 0 {
                    queue.push_back(child);
                }
            }
        }
        debug_assert_eq!(self.order.len(), num_nodes)
    }
}
