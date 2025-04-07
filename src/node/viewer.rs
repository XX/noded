use eframe::egui_wgpu::RenderState;
use egui::emath::Numeric;
use egui::epaint::Hsva;
use egui::{Color32, Ui, WidgetText};
use egui_snarl::ui::{AnyPins, PinInfo, SnarlViewer, WireStyle};
use egui_snarl::{InPin, InPinId, NodeId, OutPin, OutPinId, Snarl};

use super::material::LambertianNode;
use super::render::raytracer::RaytracerRenderNode;
use super::render::triangle::TriangleRenderNode;
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

pub struct NodeConfig {
    pub render_state: RenderState,
    pub max_viewport_resolution: u32,
}

pub struct NodeViewer {
    config: NodeConfig,
    render: Option<NodeId>,
}

impl NodeViewer {
    pub fn new(render_state: RenderState, max_viewport_resolution: u32, snarl: &Snarl<Node>) -> Self {
        let mut render = None;

        for (from_pin, to_pin) in snarl.wires() {
            if snarl[to_pin.node].output_node_ref().is_some() {
                if let Some(render_node) = snarl[from_pin.node].render_node_ref() {
                    render_node.register(&render_state);
                    render = Some(from_pin.node);
                }
            }
        }

        Self {
            render,
            config: NodeConfig {
                render_state,
                max_viewport_resolution,
            },
        }
    }

    pub fn draw(&mut self, viewport: &egui::Rect, painter: &egui::Painter, snarl: &mut Snarl<Node>) {
        if let Some(id) = self.render {
            match snarl.get_node(id).and_then(Node::render_node_ref) {
                Some(RenderNode::Triangle(render)) => {
                    render.draw(*viewport, painter);
                },
                Some(RenderNode::Raytracer(render)) => {
                    render.draw(*viewport, painter, snarl);
                },
                None => (),
            }
        }
    }

    pub fn after_show(&mut self, ui: &mut Ui, response: &egui::Response, snarl: &mut Snarl<Node>) {
        if let Some(id) = self.render {
            match snarl[id].as_render_node_mut() {
                RenderNode::Triangle(render) => {
                    let drag = response.drag_delta().x;
                    render.recalc_angle(drag as _);
                },
                RenderNode::Raytracer(render) => {
                    // let camera_id = render.camera_id();
                    if let Some(camera) = render
                        .camera_id()
                        .and_then(|camera_id| snarl.get_node_mut(camera_id).and_then(Node::camera_node_mut))
                    {
                        ui.input(|i| camera.after_events(i));
                    }
                    // let drag = response. drag_delta().x;
                },
            }
        }
    }

    fn unregister_render(&mut self, snarl: &mut Snarl<Node>) {
        if let Some(id) = self.render.take() {
            if let Some(render_node) = snarl.get_node(id).and_then(Node::render_node_ref) {
                render_node.unregister(&self.config.render_state);
            }
        }
    }
}

impl SnarlViewer<Node> for NodeViewer {
    #[inline]
    fn connect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<Node>) {
        // Validate connection
        if snarl[from.id.node].outputs()[from.id.output] & snarl[to.id.node].inputs()[to.id.input] != 0 {
            for &remote in &to.remotes {
                let out_pin = snarl.out_pin(remote);
                self.disconnect(&out_pin, to, snarl);
            }

            snarl.connect(from.id, to.id);
            snarl[to.id.node].connect_input(from, to);
            // snarl[from.id.node].connect_output(from, to);

            if snarl[to.id.node].output_node_ref().is_some() {
                if let Some(render_node) = snarl[from.id.node].render_node_ref() {
                    render_node.register(&self.config.render_state);
                    self.render = Some(from.id.node);
                }
            }
        }
    }

    #[inline]
    fn disconnect(&mut self, from: &OutPin, to: &InPin, snarl: &mut Snarl<Node>) {
        snarl[to.id.node].disconnect_input(to);
        snarl.disconnect(from.id, to.id);

        if self.render == Some(from.id.node) {
            self.unregister_render(snarl);
        }

        if let Some(collection_node) = snarl[to.id.node].collection_node_ref() {
            // Reconnect rest of inputs
            for input in (to.id.input + 1)..collection_node.inputs().len() {
                let to_pin_id = InPinId {
                    node: to.id.node,
                    input,
                };
                let to_pin = snarl.in_pin(to_pin_id);

                for from_pin_id in to_pin.remotes {
                    snarl.disconnect(from_pin_id, to_pin_id);
                    snarl.connect(from_pin_id, InPinId {
                        node: to.id.node,
                        input: input - 1,
                    });
                }
            }
        }
    }

    #[inline]
    fn drop_inputs(&mut self, pin: &InPin, snarl: &mut Snarl<Node>) {
        println!("Dropping input");
        // FIXME: where is this called?
        snarl[pin.id.node].disconnect_input(pin);
        snarl.drop_inputs(pin.id);
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
        match snarl[pin.id.node] {
            Node::Material(MaterialNode::Metal(_)) => MetalNode::show_input(pin, ui, snarl),
            Node::Material(MaterialNode::Dielectric(_)) => DielectricNode::show_input(pin, ui, snarl),
            Node::Material(MaterialNode::Lambertian(_)) => LambertianNode::show_input(pin, ui, snarl),
            Node::Primitive(PrimitiveNode::Sphere(_)) => SphereNode::show_input(pin, ui, snarl),
            Node::Collection(ref collection) => collection.show_input(pin, ui, snarl),
            Node::Camera(_) => CameraNode::show_input(pin, ui, snarl),
            Node::Render(RenderNode::Triangle(_)) => TriangleRenderNode::show_input(pin, ui, snarl),
            Node::Render(RenderNode::Raytracer(_)) => RaytracerRenderNode::show_input(pin, ui, snarl),
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
                snarl.insert_node(pos, factory(&self.config));
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
            AnyPins::Out(src_pin_ids) => {
                for src_pin_id in src_pin_ids {
                    let src_out = snarl[src_pin_id.node].outputs()[src_pin_id.output];
                    let dst_in_candidates = Node::fabrics().into_iter().filter_map(|(name, factory, inputs, _)| {
                        inputs
                            .iter()
                            .position(|input| *input & src_out != 0)
                            .map(|idx| (name, factory, idx))
                    });

                    for (name, factory, idx) in dst_in_candidates {
                        if ui.button(name).clicked() {
                            // Create new node.
                            let node = snarl.insert_node(pos, factory(&self.config));

                            // Connect the wire.
                            let src_pin = snarl.out_pin(*src_pin_id);
                            let dst_pin = InPin {
                                id: InPinId { node, input: idx },
                                remotes: Default::default(),
                            };
                            self.connect(&src_pin, &dst_pin, snarl);

                            ui.close_menu();
                        }
                    }
                }
            },
            AnyPins::In(src_pin_ids) => {
                for src_pin_id in src_pin_ids {
                    let src_in = snarl[src_pin_id.node].inputs()[src_pin_id.input];
                    let dst_out_candidates = Node::fabrics().into_iter().filter_map(|(name, factory, _, outputs)| {
                        outputs
                            .iter()
                            .position(|output| *output & src_in != 0)
                            .map(|idx| (name, factory, idx))
                    });

                    for (name, factory, idx) in dst_out_candidates {
                        if ui.button(name).clicked() {
                            // Create new node.
                            let node = snarl.insert_node(pos, factory(&self.config));

                            // Connect the wire.
                            let dst_pin = OutPin {
                                id: OutPinId { node, output: idx },
                                remotes: Default::default(),
                            };
                            let src_pin = snarl.in_pin(*src_pin_id);
                            self.connect(&dst_pin, &src_pin, snarl);

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
        node_id: NodeId,
        inputs: &[InPin],
        outputs: &[OutPin],
        ui: &mut Ui,
        snarl: &mut Snarl<Node>,
    ) {
        ui.label("Node menu");
        if ui.button("Remove").clicked() {
            if self.render == Some(node_id) {
                self.unregister_render(snarl);
            }

            for in_pin in inputs {
                for out_pin_id in &in_pin.remotes {
                    let out_pin = snarl.out_pin(*out_pin_id);
                    self.disconnect(&out_pin, &in_pin, snarl);
                }
            }

            for out_pin in outputs {
                for in_pin_id in &out_pin.remotes {
                    let in_pin = snarl.in_pin(*in_pin_id);
                    self.disconnect(out_pin, &in_pin, snarl);
                }
            }

            snarl.remove_node(node_id);

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

pub fn number_input_view<N>(
    ui: &mut Ui,
    label: &str,
    node_pin: &mut NodePin<N>,
    remote_value: Option<(&'static str, N)>,
) -> PinInfo
where
    N: Numeric,
{
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

pub fn as_number_input_view<N, M>(
    ui: &mut Ui,
    label: &str,
    node_pin: &mut NodePin<N>,
    remote_value: Option<(&'static str, M)>,
) -> PinInfo
where
    N: AsMut<f64>,
    M: Into<N>,
{
    ui.horizontal(|ui| {
        ui.label(label);
        let enabled = match remote_value {
            None => true,
            Some(remote) => {
                node_pin.set(remote.1.into());
                false
            },
        };
        ui.add_enabled(enabled, egui::DragValue::new(node_pin.as_mut().as_mut()));
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

pub fn empty_input_view(ui: &mut Ui, label: impl Into<WidgetText>) -> PinInfo {
    ui.label(label);
    PinInfo::circle().with_fill(UNTYPED_COLOR)
}
