use std::{fmt::Display, path::Path};

use iced::{
    alignment::Vertical::Top,
    widget::{pick_list, scrollable, Row},
    Element,
    Length::Fill,
};
use musolver::{
    mus::Accion,
    solver::{Strategy, TipoEstrategia},
    ActionNode,
};

pub struct SquareData {
    pub v: Vec<(f32, f32, f32)>,
}

#[derive(Clone, Debug)]
enum AppEvent {
    SetAction(usize, OptionalAction),
    SetStrategy(TipoEstrategia),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OptionalAction(Option<Accion>);

impl Display for OptionalAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(a) => write!(f, "{}", a),
            None => write!(f, ""),
        }
    }
}

#[derive(Debug)]
pub struct ActionPath {
    pub action_tree: ActionNode<usize, Accion>,
    pub selected_strategy: Option<TipoEstrategia>,
    pub strategies: Vec<TipoEstrategia>,
    pub selected_actions: Vec<Option<OptionalAction>>,
    pub actions: Vec<Vec<OptionalAction>>,
}

impl Default for ActionPath {
    fn default() -> Self {
        ActionPath::new()
    }
}

impl ActionPath {
    fn new() -> Self {
        let strategy = Strategy::from_file(Path::new("output/2024-10-05 14:13/Punto.json"))
            .expect("Error cargando estrategia.");
        let action_tree: ActionNode<usize, Accion> =
            strategy.strategy_config.trainer_config.action_tree;
        let children = action_tree.children().unwrap();
        let mut valores: Vec<OptionalAction> = vec![OptionalAction(None)];
        valores.extend(children.iter().map(|c| OptionalAction(Some(c.0))));
        Self {
            action_tree,
            selected_actions: vec![None],
            actions: vec![valores],
            selected_strategy: Some(TipoEstrategia::DosManos),
            strategies: vec![
                TipoEstrategia::DosManos,
                TipoEstrategia::TresManos1vs2,
                TipoEstrategia::TresManos1vs2Intermedio,
                TipoEstrategia::TresManos2vs1,
                TipoEstrategia::CuatroManos,
            ],
        }
    }

    fn update(&mut self, message: AppEvent) {
        match message {
            AppEvent::SetAction(level, action) => {
                self.selected_actions[level] = action.0.map(|_| action);
                self.selected_actions.drain(level + 1..);
                self.actions.drain(level + 1..);

                if action.0.is_some() {
                    let mut current_node = &self.action_tree;
                    for s in &self.selected_actions {
                        if let Some(a) = s {
                            current_node = current_node.next_node(a.0.unwrap()).unwrap();
                        } else {
                            break;
                        }
                    }
                    if let ActionNode::NonTerminal(_, children) = &current_node {
                        let mut valores: Vec<OptionalAction> = vec![OptionalAction(None)];
                        valores.extend(children.iter().map(|c| OptionalAction(Some(c.0))));
                        self.selected_actions.push(None);
                        self.actions.push(valores);
                    }
                }
            }
            AppEvent::SetStrategy(strategy) => self.selected_strategy = Some(strategy),
        }
    }

    fn view(&self) -> Element<AppEvent> {
        let mut top_row = Row::new();

        let pick_strategy = pick_list(
            &self.strategies[..],
            self.selected_strategy,
            AppEvent::SetStrategy,
        )
        .placeholder("Select a strategy");
        top_row = top_row.push(pick_strategy);

        let pick_action1 = pick_list(&self.actions[0][..], self.selected_actions[0], |elem| {
            AppEvent::SetAction(0, elem)
        })
        .placeholder("Select an action");
        top_row = top_row.push(pick_action1);

        if self.selected_actions.len() > 1 {
            for level in 1..self.selected_actions.len() {
                let pick_action_n = pick_list(
                    &self.actions[level][..],
                    self.selected_actions[level],
                    move |elem| AppEvent::SetAction(level, elem),
                )
                .placeholder("Select an action");
                top_row = top_row.push(pick_action_n);
            }
        }
        top_row = top_row.width(Fill).align_y(Top).spacing(10);

        scrollable(top_row).into()
    }
}

// struct Square {}

// impl Square {
//     pub fn new<L>(cx: &mut Context, v1: L) -> Handle<Self>
//     where
//         L: Lens<Target = (f32, f32, f32)>,
//     {
//         Self {}.build(cx, |cx| {
//             let sum = v1.map(|v| {
//                 let sum = v.0 + v.1 + v.2;
//                 (
//                     Percentage(100. * v.0 / sum),
//                     Percentage(100. * v.1 / sum),
//                     Percentage(100. * v.2 / sum),
//                 )
//             });
//             HStack::new(cx, |cx| {
//                 Element::new(cx)
//                     .background_color(Color::rgb(255, 0, 0))
//                     .width(sum.map(|v| v.0));
//                 Element::new(cx)
//                     .background_color(Color::rgb(0, 255, 0))
//                     .width(sum.map(|v| v.1));
//                 Element::new(cx)
//                     .background_color(Color::rgb(0, 0, 255))
//                     .width(sum.map(|v| v.2));
//             })
//             .width(Units::Pixels(100.))
//             .height(Units::Pixels(100.));
//         })
//     }
// }

// impl View for Square {}

fn main() {
    let _ = iced::run("Inspector", ActionPath::update, ActionPath::view);
    // let _ = Application::new(|cx| {
    //     SquareData {
    //         v: vec![(0.2, 0.1, 0.4), (0.5, 0.2, 0.4)],
    //     }
    //     .build(cx);
    //     ActionPath::new().build(cx);
    //     HStack::new(cx, |cx| {
    //         Square::new(cx, SquareData::v.idx(0));
    //         Square::new(cx, SquareData::v.idx(1));
    //     });
    // })
    // .title("Counter")
    // .inner_size((400, 400))
    // .run();
}
