use egui::epaint::Hsva;
use egui::{Color32, Ui};
use egui_snarl::ui::{AnyPins, PinInfo, SnarlViewer, WireStyle};
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};

use super::material::LambertNode;
use super::{
    CameraNode, DielectricNode, MaterialNode, MetalNode, Node, OutputNode, PrimitiveNode, RenderNode, SphereNode,
};
use crate::node::expression::ExpressionNode;
use crate::types::{Color, NodePin, Vector3};
use crate::widget::color_picker::{Alpha, color_button, color_edit_button_srgba};

pub const STRING_COLOR: Color32 = Color32::from_rgb(0x00, 0xb0, 0x00);
pub const NUMBER_COLOR: Color32 = Color32::from_rgb(0xb0, 0x00, 0x00);
pub const VECTOR_COLOR: Color32 = Color32::from_rgb(0x00, 0x00, 0xb0);
pub const MATERIAL_COLOR: Color32 = Color32::from_rgb(0xb0, 0x00, 0xb0);
pub const UNTYPED_COLOR: Color32 = Color32::from_rgb(0xb0, 0xb0, 0xb0);

pub struct NodeViewer;

impl SnarlViewer<Node> for NodeViewer {
    #[inline]
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<Node>) {
        // Validate connection
        if snarl[from.id.node].outputs()[from.id.output] & snarl[to.id.node].inputs()[to.id.input] != 0 {
            for &remote in &to.remotes {
                snarl.disconnect(remote, to.id);
            }

            snarl.connect(from.id, to.id);
        }
    }

    fn title(&mut self, node: &Node) -> String {
        node.name().to_owned()
    }

    fn inputs(&mut self, node: &Node) -> usize {
        node.inputs().len()
    }

    fn outputs(&mut self, node: &Node) -> usize {
        node.outputs().len()
    }

    #[allow(refining_impl_trait)]
    fn show_input(&mut self, pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match &mut snarl[pin.id.node] {
            Node::Material(MaterialNode::Metal(_)) => MetalNode::show_input(pin, ui, snarl),
            Node::Material(MaterialNode::Dielectric(_)) => DielectricNode::show_input(pin, ui, snarl),
            Node::Material(MaterialNode::Lambert(_)) => LambertNode::show_input(pin, ui, snarl),
            Node::Primitive(PrimitiveNode::Sphere(_)) => SphereNode::show_input(pin, ui, snarl),
            Node::Collection(collection) => collection.show_input(pin, ui),
            Node::Camera(_) => CameraNode::show_input(pin, ui, snarl),
            Node::Render(_) => RenderNode::show_input(pin, ui, snarl),
            Node::Output(_) => OutputNode::show_input(pin, ui, snarl),
            Node::Number(_) => {
                unreachable!("{} node has no inputs", Node::NUMBER_NAME)
            },
            Node::String(_) => {
                unreachable!("{} node has no inputs", Node::STRING_NAME)
            },
            Node::Color(_) => {
                unreachable!("{} node has no inputs", Node::COLOR_NAME)
            },
            Node::Vector(_) => {
                unreachable!("{} node has no inputs", Node::VECTOR_NAME)
            },
            Node::Expression(_) => ExpressionNode::show_input(pin, ui, snarl),
        }
    }

    #[allow(refining_impl_trait)]
    fn show_output(&mut self, pin: &OutPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match &mut snarl[pin.id.node] {
            Node::Material(_) => PinInfo::circle().with_fill(MATERIAL_COLOR),
            Node::Output(_) => {
                unreachable!("Output node has no outputs")
            },
            Node::Number(value) => {
                assert_eq!(pin.id.output, 0, "Number node has only one output");
                ui.add(egui::DragValue::new(value));
                PinInfo::circle().with_fill(NUMBER_COLOR)
            },
            Node::String(value) => {
                assert_eq!(pin.id.output, 0, "String node has only one output");
                let edit = egui::TextEdit::singleline(value)
                    .clip_text(false)
                    .desired_width(0.0)
                    .margin(ui.spacing().item_spacing);
                ui.add(edit);
                PinInfo::circle()
                    .with_fill(STRING_COLOR)
                    .with_wire_style(WireStyle::AxisAligned { corner_radius: 10.0 })
            },
            Node::Color(value) => {
                assert_eq!(pin.id.output, 0, "Color node has only one output");
                color_edit_button_srgba(ui, value, Alpha::BlendOrAdditive);
                PinInfo::circle().with_fill(NUMBER_COLOR)
            },
            Node::Vector(vector) => {
                assert_eq!(pin.id.output, 0, "Number node has only one output");
                ui.add(egui::DragValue::new(&mut vector[0]));
                ui.add(egui::DragValue::new(&mut vector[1]));
                ui.add(egui::DragValue::new(&mut vector[2]));
                PinInfo::circle().with_fill(VECTOR_COLOR)
            },
            Node::Expression(expr_node) => {
                let value = expr_node.eval();
                assert_eq!(pin.id.output, 0, "Expr node has only one output");
                ui.label(format_float(value));
                PinInfo::circle().with_fill(NUMBER_COLOR)
            },
            _ => PinInfo::circle().with_fill(UNTYPED_COLOR),
        }
    }

    fn has_graph_menu(&mut self, _pos: egui::Pos2, _snarl: &mut Snarl<Node>) -> bool {
        true
    }

    fn show_graph_menu(&mut self, pos: egui::Pos2, ui: &mut Ui, snarl: &mut Snarl<Node>) {
        ui.label("Add node");
        for (name, factory, ..) in Node::fabrics() {
            if ui.button(name).clicked() {
                snarl.insert_node(pos, factory());
                ui.close_menu();
            }
        }
    }

    fn has_dropped_wire_menu(&mut self, _src_pins: AnyPins, _snarl: &mut Snarl<Node>) -> bool {
        true
    }

    fn show_dropped_wire_menu(&mut self, pos: egui::Pos2, ui: &mut Ui, src_pins: AnyPins, snarl: &mut Snarl<Node>) {
        ui.label("Add node");
        match src_pins {
            AnyPins::Out(src_pins) => {
                for src_pin in src_pins {
                    let src_out = snarl[src_pin.node].outputs()[src_pin.output];
                    let dst_in_candidates = Node::fabrics().into_iter().filter_map(|(name, factory, inputs, _)| {
                        inputs
                            .iter()
                            .position(|input| *input & src_out != 0)
                            .map(|idx| (name, factory, idx))
                    });

                    for (name, factory, idx) in dst_in_candidates {
                        if ui.button(name).clicked() {
                            // Create new node.
                            let node = snarl.insert_node(pos, factory());
                            let dst_pin = InPinId { node, input: idx };

                            // Connect the wire.
                            snarl.connect(*src_pin, dst_pin);
                            ui.close_menu();
                        }
                    }
                }
            },
            AnyPins::In(src_pins) => {
                for src_pin in src_pins {
                    let src_in = snarl[src_pin.node].inputs()[src_pin.input];
                    let dst_out_candidates = Node::fabrics().into_iter().filter_map(|(name, factory, _, outputs)| {
                        outputs
                            .iter()
                            .position(|output| *output & src_in != 0)
                            .map(|idx| (name, factory, idx))
                    });

                    for (name, factory, idx) in dst_out_candidates {
                        if ui.button(name).clicked() {
                            // Create new node.
                            let node = snarl.insert_node(pos, factory());
                            let dst_pin = OutPinId { node, output: idx };

                            // Connect the wire.
                            // snarl.drop_inputs(*src_pin);
                            snarl.connect(dst_pin, *src_pin);
                            ui.close_menu();
                        }
                    }
                }
            },
        };
    }

    fn has_node_menu(&mut self, _node: &Node) -> bool {
        true
    }

    fn show_node_menu(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<Node>,
    ) {
        ui.label("Node menu");
        if ui.button("Remove").clicked() {
            snarl.remove_node(node);
            ui.close_menu();
        }
    }

    fn has_on_hover_popup(&mut self, _: &Node) -> bool {
        true
    }

    fn show_on_hover_popup(
        &mut self,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<Node>,
    ) {
        match snarl[node] {
            Node::Output(_) => {
                ui.label("Displays anything connected to it");
            },
            Node::Number(_) => {
                ui.label("Outputs integer value");
            },
            Node::String(_) => {
                ui.label("Outputs string value");
            },
            Node::Expression(_) => {
                ui.label("Evaluates algebraic expression with input for each unique variable name");
            },
            _ => {
                ui.label("<No description available>");
            },
        }
    }

    fn header_frame(
        &mut self,
        frame: egui::Frame,
        node: NodeId,
        _inputs: &[InPin],
        _outputs: &[OutPin],
        snarl: &Snarl<Node>,
    ) -> egui::Frame {
        match snarl[node] {
            Node::Output(_) => frame.fill(egui::Color32::from_rgb(70, 70, 80)),
            Node::Number(_) => frame.fill(egui::Color32::from_rgb(70, 40, 40)),
            Node::String(_) => frame.fill(egui::Color32::from_rgb(40, 70, 40)),
            Node::Expression(_) => frame.fill(egui::Color32::from_rgb(70, 66, 40)),
            _ => frame.fill(egui::Color32::from_rgb(40, 40, 70)),
        }
    }
}

pub fn format_float(value: f64) -> String {
    let value = (value * 1000.0).round() / 1000.0;
    format!("{value}")
}

pub fn number_input_remote_value(pin: &InPin, snarl: &Snarl<Node>, label: &str) -> Option<(&'static str, f64)> {
    match &*pin.remotes {
        [] => None,
        [remote] => Some(match &snarl[remote.node] {
            Node::Number(value) => (Node::NUMBER_NAME, *value),
            Node::Expression(expr) => (ExpressionNode::NAME, expr.eval()),
            node => unreachable!("{label} input not suppor connection with `{}`", node.name()),
        }),
        _ => None,
    }
}

pub fn number_input_view(
    ui: &mut Ui,
    label: &str,
    node_pin: &mut NodePin<f64>,
    remote_value: Option<(&'static str, f64)>,
) -> PinInfo {
    ui.horizontal(|ui| {
        ui.label(label);
        let enabled = match remote_value {
            None => true,
            Some(remote) => {
                node_pin.set(remote.1);
                false
            },
        };
        ui.add_enabled(enabled, egui::DragValue::new(node_pin.as_mut()));
    });
    PinInfo::circle().with_fill(NUMBER_COLOR)
}

pub fn vector_input_remote_value(pin: &InPin, snarl: &Snarl<Node>, label: &str) -> Option<(&'static str, Vector3)> {
    match &*pin.remotes {
        [] => None,
        [remote] => Some(match &snarl[remote.node] {
            Node::Number(value) => (Node::NUMBER_NAME, Vector3::new(*value, *value, *value)),
            Node::Vector(vector) => (Node::VECTOR_NAME, *vector),
            Node::Color(color) => (
                Node::COLOR_NAME,
                Vector3::new(color[0] as _, color[1] as _, color[2] as _),
            ),
            Node::Expression(expr) => {
                let value = expr.eval();
                (ExpressionNode::NAME, Vector3::new(value, value, value))
            },
            node => unreachable!("{label} input not suppor connection with `{}`", node.name()),
        }),
        _ => None,
    }
}

pub fn vector_input_view(
    ui: &mut Ui,
    label: &str,
    node_pin: &mut NodePin<Vector3>,
    remote_value: Option<(&'static str, Vector3)>,
) -> PinInfo {
    ui.horizontal(|ui| {
        ui.label(label);
        let enabled = match remote_value {
            None => true,
            Some(remote) => {
                node_pin.set(remote.1);
                false
            },
        };
        ui.add_enabled(enabled, egui::DragValue::new(&mut node_pin.as_mut()[0]));
        ui.add_enabled(enabled, egui::DragValue::new(&mut node_pin.as_mut()[1]));
        ui.add_enabled(enabled, egui::DragValue::new(&mut node_pin.as_mut()[2]));
    });
    PinInfo::circle().with_fill(VECTOR_COLOR)
}

pub fn color_input_remote_value(pin: &InPin, snarl: &Snarl<Node>, label: &str) -> Option<(&'static str, Color)> {
    match &*pin.remotes {
        [] => None,
        [remote] => Some(match &snarl[remote.node] {
            Node::Number(value) => (Node::NUMBER_NAME, Color::from_gray((*value * 255.0).round() as u8)),
            Node::Color(color) => (Node::COLOR_NAME, *color),
            Node::Vector(vector) => (
                Node::VECTOR_NAME,
                Color::from_rgb(
                    (vector.x * 255.0).round() as u8,
                    (vector.y * 255.0).round() as u8,
                    (vector.z * 255.0).round() as u8,
                ),
            ),
            Node::Expression(expr) => {
                let value = expr.eval();
                (ExpressionNode::NAME, Color::from_gray((value * 255.0).round() as u8))
            },
            node => unreachable!("{label} input not suppor connection with `{}`", node.name()),
        }),
        _ => None,
    }
}

pub fn color_input_view(
    ui: &mut Ui,
    label: &str,
    node_pin: &mut NodePin<Color>,
    remote_value: Option<(&'static str, Color)>,
) -> PinInfo {
    ui.horizontal(|ui| {
        ui.label(label);
        match remote_value {
            None => {
                color_edit_button_srgba(ui, node_pin.as_mut(), Alpha::BlendOrAdditive);
            },
            Some(remote) => {
                let show_color_button = match remote.0 {
                    Node::NUMBER_NAME => true,
                    Node::COLOR_NAME => false,
                    Node::VECTOR_NAME => true,
                    ExpressionNode::NAME => true,
                    node => unreachable!("{label} input not suppor connection with `{node}`"),
                };

                node_pin.set(remote.1);

                if show_color_button {
                    color_button(ui, Hsva::from(node_pin.get()).into(), false);
                }
            },
        }
    });
    PinInfo::circle().with_fill(node_pin.get())
}

pub fn material_input_remote_value(
    pin: &InPin,
    snarl: &Snarl<Node>,
    label: &str,
) -> Option<(&'static str, MaterialNode)> {
    match &*pin.remotes {
        [] => None,
        [remote] => Some(match &snarl[remote.node] {
            Node::Material(material) => (material.name(), material.clone()),
            node => unreachable!("{label} input not suppor connection with `{}`", node.name()),
        }),
        _ => None,
    }
}

pub fn material_input_view(
    ui: &mut Ui,
    label: &str,
    node_pin: &mut NodePin<MaterialNode>,
    remote_value: Option<(&'static str, MaterialNode)>,
) -> PinInfo {
    ui.horizontal(|ui| {
        ui.label(label);
        match remote_value {
            None => {}, //material_select(ui, node_pin.as_mut()),
            Some(remote) => {
                node_pin.set(remote.1);
                //material_select(ui, node_pin.as_mut());
            },
        }
    });
    PinInfo::circle().with_fill(MATERIAL_COLOR)
}

pub fn empty_input_view(ui: &mut Ui, label: &str) -> PinInfo {
    ui.label(label);
    PinInfo::circle().with_fill(UNTYPED_COLOR)
}
