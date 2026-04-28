use std::collections::{HashMap, VecDeque, hash_map::Entry};

use arrayvec::{ArrayString, ArrayVec};

use crate::Game;

use super::NodeType;

#[derive(Debug)]
pub struct GameNode<G, D> {
    game: G,
    next_nodes: ArrayVec<usize, 16>,
    info_set_str: Option<ArrayString<64>>,
    data: D,
}

impl<G, D> GameNode<G, D> {
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

    pub fn info_set_str(&self) -> Option<&str> {
        self.info_set_str.as_ref().map(|info_set| info_set.as_str())
    }

    pub fn children(&self) -> &[usize] {
        &self.next_nodes
    }
}

#[derive(Debug)]
pub struct GameGraph<G, D> {
    node_ids: HashMap<String, usize>,
    game_nodes: Vec<GameNode<G, D>>,
}

impl<G, D> GameGraph<G, D>
where
    G: Game + Clone,
    G::Action: Copy,
    D: Default,
{
    pub fn new(game: G) -> Self {
        let history_str = game.history_str();
        let mut node_ids = HashMap::from([(history_str, 0)]);
        node_ids.reserve(5000);
        let current_node = game.current_player();
        let info_set_str = match current_node {
            NodeType::Chance | NodeType::Terminal => None,
            NodeType::Player(player_id) => ArrayString::from(&game.info_set_str(player_id)).ok(),
        };
        let mut game_nodes = Vec::with_capacity(512);
        game_nodes.push(GameNode {
            game,
            next_nodes: ArrayVec::new(),
            info_set_str,
            data: D::default(),
        });

        Self {
            node_ids,
            game_nodes,
        }
    }

    pub fn inflate(&mut self) {
        let mut game_list = VecDeque::from([0]);
        while let Some(parent_idx) = game_list.pop_front() {
            self.next_nodes(parent_idx, &mut game_list);
        }
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

    fn next_nodes(&mut self, idx: usize, new_nodes: &mut VecDeque<usize>) {
        let game = &self.game_nodes[idx].game;
        match game.current_player() {
            NodeType::Chance => {
                let mut new_game = self.game_nodes[idx].game.clone();
                new_game.new_random();

                if let Some(child_idx) = self.append_child(idx, new_game) {
                    new_nodes.push_back(child_idx);
                }
            }
            NodeType::Player(_) => {
                let actions = game.actions();
                new_nodes.extend(actions.iter().filter_map(|action| {
                    let mut new_game = self.game_nodes[idx].game.clone();
                    new_game.act(*action);
                    self.append_child(idx, new_game)
                }))
            }
            NodeType::Terminal => {}
        }
    }

    fn append_child(&mut self, parent_idx: usize, new_game: G) -> Option<usize> {
        let history_str = new_game.history_str();
        match self.node_ids.entry(history_str) {
            Entry::Occupied(next_id) => {
                self.game_nodes[parent_idx].next_nodes.push(*next_id.get());
                None
            }
            Entry::Vacant(vacant_entry) => {
                let current_node = new_game.current_player();
                let info_set_str = match current_node {
                    NodeType::Chance | NodeType::Terminal => None,
                    NodeType::Player(player_id) => {
                        ArrayString::from(&new_game.info_set_str(player_id)).ok()
                    }
                };
                let last_node_id = self.game_nodes.len();
                vacant_entry.insert(last_node_id);
                self.game_nodes.push(GameNode {
                    game: new_game,
                    next_nodes: ArrayVec::new(),
                    info_set_str,
                    data: D::default(),
                });
                self.game_nodes[parent_idx].next_nodes.push(last_node_id);
                Some(last_node_id)
            }
        }
    }
}
