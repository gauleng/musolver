use std::collections::HashMap;

use arrayvec::{ArrayString, ArrayVec};

use crate::Game;

#[derive(Debug)]
pub struct GameNode<G, D> {
    lance_game: G,
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
        &mut self.lance_game
    }

    pub fn game(&self) -> &G {
        &self.lance_game
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
    last_node_id: usize,
    game_nodes: Vec<GameNode<G, D>>,
}

impl<G, D> GameGraph<G, D>
where
    G: Game + Clone,
    G::A: Copy,
    D: Default,
{
    pub fn new(game: G) -> Self {
        let history_str = game.history_str();
        let node_ids = HashMap::from([(history_str, 0)]);
        let current_player = game.current_player().unwrap();
        let info_set_str = game.info_set_str(current_player);
        let mut game_nodes = Vec::with_capacity(512);
        game_nodes.push(GameNode {
            lance_game: game,
            next_nodes: ArrayVec::new(),
            info_set_str: ArrayString::from(&info_set_str).ok(),
            data: D::default(),
        });

        Self {
            node_ids,
            last_node_id: 0,
            game_nodes,
        }
    }

    pub fn inflate(&mut self) {
        let mut game_list = vec![0];
        while !game_list.is_empty() {
            game_list = game_list
                .drain(..)
                .flat_map(|idx| self.next_nodes(idx))
                .collect();
        }
    }

    pub fn nodes(&self) -> &[GameNode<G, D>] {
        &self.game_nodes
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

    fn next_nodes(&mut self, idx: usize) -> Vec<usize> {
        let game = &self.game_nodes[idx].lance_game;
        if game.is_terminal() {
            vec![]
        } else {
            let actions = game.actions();
            actions
                .iter()
                .filter_map(|action| {
                    let mut new_game = self.game_nodes[idx].lance_game.clone();
                    new_game.act(*action);
                    let history_str = new_game.history_str();
                    match self.node_ids.get(&history_str) {
                        Some(next_id) => {
                            self.game_nodes[idx].next_nodes.push(*next_id);
                            None
                        }
                        None => {
                            let info_set_str =
                                new_game.current_player().and_then(|current_player| {
                                    ArrayString::<64>::from(&new_game.info_set_str(current_player))
                                        .ok()
                                });
                            self.last_node_id += 1;
                            self.node_ids.insert(history_str, self.last_node_id);
                            self.game_nodes.push(GameNode {
                                lance_game: new_game,
                                next_nodes: ArrayVec::new(),
                                info_set_str,
                                data: D::default(),
                            });
                            self.game_nodes[idx].next_nodes.push(self.last_node_id);
                            Some(self.last_node_id)
                        }
                    }
                })
                .collect()
        }
    }
}
