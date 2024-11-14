use musolver::{
    mus::{Accion, Lance},
    solver::LanceGame,
    Game, Node,
};
use std::collections::HashMap;

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
    // println!("{:?}", nodes);
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
