use std::path::Path;

use musolver::{
    mus::Accion,
    solver::{Strategy, TipoEstrategia},
    ActionNode,
};
use vizia::prelude::*;

#[derive(Lens)]
pub struct SquareData {
    pub v: Vec<(f32, f32, f32)>,
}

impl Model for SquareData {}

#[derive(Lens)]
pub struct ActionPath {
    pub action_tree: ActionNode<usize, Accion>,
    pub selected_actions: Vec<usize>,
    pub valores: Vec<Vec<String>>,
    pub selected_strategy: usize,
    pub strategies: Vec<String>,
}

impl ActionPath {
    fn new() -> Self {
        let action_tree: ActionNode<usize, Accion> =
            ActionNode::from_file(Path::new("config/action_tree.json"))
                .expect("Error loading action tree.");
        if let ActionNode::NonTerminal(_, children) = &action_tree {
            let mut valores: Vec<String> = children.iter().map(|c| c.0.to_string()).collect();
            valores.insert(0, "".to_string());
            Self {
                action_tree,
                selected_actions: vec![0],
                valores: vec![valores],
                selected_strategy: 0,
                strategies: vec![
                    TipoEstrategia::DosManos.to_string(),
                    TipoEstrategia::TresManos1vs2.to_string(),
                    TipoEstrategia::TresManos1vs2Intermedio.to_string(),
                    TipoEstrategia::TresManos2vs1.to_string(),
                    TipoEstrategia::CuatroManos.to_string(),
                ],
            }
        } else {
            Self {
                action_tree,
                selected_actions: vec![0],
                valores: Vec::new(),
                selected_strategy: 0,
                strategies: vec![
                    TipoEstrategia::DosManos.to_string(),
                    TipoEstrategia::TresManos1vs2.to_string(),
                    TipoEstrategia::TresManos1vs2Intermedio.to_string(),
                    TipoEstrategia::TresManos2vs1.to_string(),
                    TipoEstrategia::CuatroManos.to_string(),
                ],
            }
        }
    }
}

impl Model for ActionPath {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            AppEvent::SetAction(level, accion) => {
                self.selected_actions[*level] = *accion;
                self.selected_actions.drain(level + 1..);
                self.valores.drain(level + 1..);

                if *accion > 0 {
                    let mut current_node = &self.action_tree;
                    for s in &self.selected_actions {
                        match current_node {
                            ActionNode::Terminal => todo!(),
                            ActionNode::NonTerminal(_, children) => {
                                current_node = &children[*s - 1].1
                            }
                        }
                    }
                    if let ActionNode::NonTerminal(_, children) = &current_node {
                        let mut valores: Vec<String> =
                            children.iter().map(|c| c.0.to_string()).collect();
                        valores.insert(0, "".to_string());
                        self.selected_actions.push(0);
                        self.valores.push(valores);
                    }
                }
            }
            AppEvent::SetStrategy(idx) => self.selected_strategy = *idx,
        });
    }
}

enum AppEvent {
    SetAction(usize, usize),
    SetStrategy(usize),
}

struct Square {}

impl Square {
    pub fn new<L>(cx: &mut Context, v1: L) -> Handle<Self>
    where
        L: Lens<Target = (f32, f32, f32)>,
    {
        Self {}.build(cx, |cx| {
            let sum = v1.map(|v| {
                let sum = v.0 + v.1 + v.2;
                (
                    Percentage(100. * v.0 / sum),
                    Percentage(100. * v.1 / sum),
                    Percentage(100. * v.2 / sum),
                )
            });
            HStack::new(cx, |cx| {
                Element::new(cx)
                    .background_color(Color::rgb(255, 0, 0))
                    .width(sum.map(|v| v.0));
                Element::new(cx)
                    .background_color(Color::rgb(0, 255, 0))
                    .width(sum.map(|v| v.1));
                Element::new(cx)
                    .background_color(Color::rgb(0, 0, 255))
                    .width(sum.map(|v| v.2));
            })
            .width(Units::Pixels(100.))
            .height(Units::Pixels(100.));
        })
    }
}

impl View for Square {}

fn main() {
    let strategy = Strategy::from_file(Path::new("output/2024-10-05 14:13/Punto.json"));
    let _ = Application::new(|cx| {
        SquareData {
            v: vec![(0.2, 0.1, 0.4), (0.5, 0.2, 0.4)],
        }
        .build(cx);
        ActionPath::new().build(cx);

        HStack::new(cx, |cx| {
            PickList::new(
                cx,
                ActionPath::strategies,
                ActionPath::selected_strategy,
                true,
            )
            .on_select(|cx, index| cx.emit(AppEvent::SetStrategy(index)))
            .width(Pixels(100.0));
            PickList::new(
                cx,
                ActionPath::valores.idx(0),
                ActionPath::selected_actions.idx(0),
                true,
            )
            .on_select(|cx, index| cx.emit(AppEvent::SetAction(0, index)))
            .width(Pixels(100.0));
            Binding::new(cx, ActionPath::selected_actions, |cx, selected| {
                if selected.get(cx).len() > 1 {
                    (1..selected.get(cx).len()).for_each(|level| {
                        PickList::new(
                            cx,
                            ActionPath::valores.idx(level),
                            ActionPath::selected_actions.idx(level),
                            true,
                        )
                        .on_select(move |cx, index| cx.emit(AppEvent::SetAction(level, index)))
                        .width(Pixels(100.0));
                    });
                }
            });
        });

        HStack::new(cx, |cx| {
            Square::new(cx, SquareData::v.idx(0));
            Square::new(cx, SquareData::v.idx(1));
        });
    })
    .title("Counter")
    .inner_size((400, 400))
    .run();
}
