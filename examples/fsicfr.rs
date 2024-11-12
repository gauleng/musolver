use musolver::{
    mus::{Accion, Lance},
    solver::LanceGame,
    Game, Node,
};
use std::collections::HashMap;

struct GameNode {
    lance_game: LanceGame,
    next_nodes: Vec<usize>,
    reach_player: f64,
    reach_opponent: f64,
    utility: f64,
}

struct GameGraph {
    node_ids: HashMap<String, usize>,
    last_node_id: usize,
    game_nodes: Vec<GameNode>,
}

impl GameGraph {
    fn new(game: LanceGame) -> Self {
        let history_str = game.history_str();
        let node_ids = HashMap::from([(history_str, 0)]);

        Self {
            node_ids,
            last_node_id: 0,
            game_nodes: vec![GameNode {
                lance_game: game,
                next_nodes: vec![],
                reach_player: 1.,
                reach_opponent: 1.,
                utility: 0.,
            }],
        }
    }

    fn inflate(&mut self) {
        let mut game_list = vec![0];
        while !game_list.is_empty() {
            game_list = game_list
                .drain(..)
                .flat_map(|idx| self.next_nodes(idx))
                .collect();
        }
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
                            let next_id = self.node_ids.entry(history_str).or_insert_with(|| {
                                self.last_node_id += 1;
                                self.game_nodes.push(GameNode {
                                    lance_game: new_game,
                                    next_nodes: vec![],
                                    reach_player: 0.,
                                    reach_opponent: 0.,
                                    utility: 0.,
                                });
                                self.last_node_id
                            });
                            self.game_nodes[idx].next_nodes.push(*next_id);
                            Some(*next_id)
                        }
                    }
                })
                .collect()
        }
    }
}

fn fsicfr(nodes: &mut HashMap<String, Node<Accion>>, game_graph: &mut GameGraph, player: usize) {
    for idx in 0..game_graph.game_nodes.len() {
        let game_node = &mut game_graph.game_nodes[idx];
        let lance_game = &mut game_node.lance_game;
        if !lance_game.is_terminal() {
            let current_player = lance_game.current_player().unwrap();
            let info_set_str = lance_game.info_set_str(current_player);
            let actions = lance_game.actions();
            let node = nodes
                .entry(info_set_str.clone())
                .or_insert_with(|| Node::new(actions.clone()));
            node.update_strategy();
            if current_player == player {
                node.update_strategy_sum(game_node.reach_player);
            }
            let strategy = node.strategy();
            for i in 0..game_graph.game_nodes[idx].next_nodes.len() {
                let child_idx = game_graph.game_nodes[idx].next_nodes[i];

                if current_player == player {
                    game_graph.game_nodes[child_idx].reach_player +=
                        strategy[i] * game_graph.game_nodes[idx].reach_player;
                    game_graph.game_nodes[child_idx].reach_opponent +=
                        game_graph.game_nodes[idx].reach_opponent;
                } else {
                    game_graph.game_nodes[child_idx].reach_player +=
                        game_graph.game_nodes[idx].reach_player;
                    game_graph.game_nodes[child_idx].reach_opponent +=
                        strategy[i] * game_graph.game_nodes[idx].reach_opponent;
                }
            }
        }
    }

    for idx in (0..game_graph.game_nodes.len()).rev() {
        let lance_game = &mut game_graph.game_nodes[idx].lance_game;
        if lance_game.is_terminal() {
            game_graph.game_nodes[idx].utility = lance_game.utility(player);
        } else {
            let current_player = lance_game.current_player().unwrap();
            let info_set_str = lance_game.info_set_str(current_player);
            let node = nodes.get_mut(&info_set_str).unwrap();
            let strategy = node.strategy();

            let utility: Vec<f64> = game_graph.game_nodes[idx]
                .next_nodes
                .iter()
                .map(|child_idx| game_graph.game_nodes[*child_idx].utility)
                .collect();
            game_graph.game_nodes[idx].utility = strategy
                .iter()
                .zip(utility.iter())
                .map(|(s, u)| s * u)
                .sum();
            if current_player == player {
                node.regret_sum
                    .iter_mut()
                    .zip(utility.iter())
                    .for_each(|(r, u)| {
                        *r += game_graph.game_nodes[idx].reach_opponent
                            * (u - game_graph.game_nodes[idx].utility)
                    });
            }
        }
        game_graph.game_nodes[idx].reach_player = 0.;
        game_graph.game_nodes[idx].reach_opponent = 0.;
    }
}

fn main() {
    let mut game = LanceGame::new(Lance::Grande, [0, 0], true);
    let mut nodes: HashMap<String, Node<Accion>> = HashMap::new();

    for _ in 0..10000 {
        game.new_random();
        let mut game_graph = GameGraph::new(game.clone());
        game_graph.inflate();
        for i in 0..4 {
            fsicfr(&mut nodes, &mut game_graph, i);
        }
    }
    println!("{:?}", nodes);
    // for node in &game_graph.game_nodes {
    //     let next_nodes = node
    //         .next_nodes
    //         .iter()
    //         .map(|idx| game_graph.game_nodes[*idx].lance_game.history_str())
    //         .collect::<Vec<String>>()
    //         .join(", ");
    //     println!(
    //         "{} ({}): {next_nodes}",
    //         node.lance_game.history_str(),
    //         node.utility,
    //     );
    // }
}
