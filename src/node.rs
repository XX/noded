use bitflags::bitflags;
use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, Snarl};
use render::raytracer::RaytracerRenderNode;
use serde::{Deserialize, Serialize};

use self::camera::CameraNode;
use self::collection::CollectionNode;
use self::expression::ExpressionNode;
use self::material::{DielectricNode, LambertianNode, MaterialNode, MetalNode};
use self::primitive::{PrimitiveNode, SphereNode};
use self::render::RenderNode;
use self::render::triangle::TriangleRenderNode;
use self::viewer::{NodeConfig, empty_input_view};
use crate::types::{Color, Vector3};

pub mod camera;
pub mod collection;
pub mod expression;
pub mod material;
pub mod primitive;
pub mod render;
pub mod viewer;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct NodeFlags: u64 {
        const MATERIAL_METAL = 0b00000001;
        const MATERIAL_DIELECTRIC = Self::MATERIAL_METAL.bits() << 1;
        const MATERIAL_LAMBERT = Self::MATERIAL_DIELECTRIC.bits() << 1;
        const MATERIALS = Self::MATERIAL_METAL.bits() | Self::MATERIAL_DIELECTRIC.bits() | Self::MATERIAL_LAMBERT.bits();

        const PRIMITIVE_SPHERE = Self::MATERIAL_LAMBERT.bits() << 1;
        const PRIMITIVES = Self::PRIMITIVE_SPHERE.bits();

        const COLLECTION = Self::PRIMITIVE_SPHERE.bits() << 1;
        const CAMERA = Self::COLLECTION.bits() << 1;

        const RENDER_TRIANGLE = Self::CAMERA.bits() << 1;
        const RENDER_RAYTRACER = Self::RENDER_TRIANGLE.bits() << 1;
        const RENDERS = Self::RENDER_TRIANGLE.bits() | Self::RENDER_RAYTRACER.bits();

        const OUTPUT = Self::RENDER_RAYTRACER.bits() << 1;

        const NUMBER = Self::OUTPUT.bits() << 1;
        const STRING = Self::NUMBER.bits() << 1;
        const COLOR = Self::STRING.bits() << 1;
        const VECTOR = Self::COLOR.bits() << 1;

        const EXPRESSION = Self::VECTOR.bits() << 1;

        const ALL = u64::MAX;
        const TYPICAL_VECTOR_INPUT = NodeFlags::VECTOR.bits() | NodeFlags::COLOR.bits() | NodeFlags::NUMBER.bits() | NodeFlags::EXPRESSION.bits();
        const TYPICAL_NUMBER_INPUT = NodeFlags::NUMBER.bits() | NodeFlags::EXPRESSION.bits();
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Node {
    Material(MaterialNode),
    Primitive(PrimitiveNode),
    Collection(CollectionNode),
    Camera(CameraNode),
    Render(RenderNode),
    Output(OutputNode),
    Number(f64),
    String(String),
    Color(Color),
    Vector(Vector3),
    Expression(ExpressionNode),
}

impl Node {
    const NUMBER_NAME: &str = "Number";
    const NUMBER_OUTPUTS: [u64; 1] = [NodeFlags::NUMBER.bits()];

    const STRING_NAME: &str = "String";
    const STRING_OUTPUTS: [u64; 1] = [NodeFlags::STRING.bits()];

    const COLOR_NAME: &str = "Color";
    const COLOR_OUTPUTS: [u64; 1] = [NodeFlags::COLOR.bits()];

    const VECTOR_NAME: &str = "Vector";
    const VECTOR_OUTPUTS: [u64; 1] = [NodeFlags::VECTOR.bits()];

    pub fn fabrics() -> impl IntoIterator<Item = (&'static str, fn(&NodeConfig) -> Node, &'static [u64], &'static [u64])>
    {
        [
            (
                MetalNode::NAME,
                (|_| Node::Material(MaterialNode::Metal(MetalNode::default()))) as fn(&NodeConfig) -> Node,
                MetalNode::INPUTS.as_slice(),
                MetalNode::OUTPUTS.as_slice(),
            ),
            (
                DielectricNode::NAME,
                |_| Node::Material(MaterialNode::Dielectric(DielectricNode::default())),
                DielectricNode::INPUTS.as_slice(),
                DielectricNode::OUTPUTS.as_slice(),
            ),
            (
                SphereNode::NAME,
                |_| Node::Primitive(PrimitiveNode::Sphere(SphereNode::default())),
                SphereNode::INPUTS.as_slice(),
                SphereNode::OUTPUTS.as_slice(),
            ),
            (
                CollectionNode::NAME,
                |_| Node::Collection(CollectionNode::default()),
                &[],
                CollectionNode::OUTPUTS.as_slice(),
            ),
            (
                CameraNode::NAME,
                |_| Node::Camera(CameraNode::default()),
                CameraNode::INPUTS.as_slice(),
                CameraNode::OUTPUTS.as_slice(),
            ),
            (
                TriangleRenderNode::NAME,
                |_| Node::Render(RenderNode::Triangle(TriangleRenderNode::default())),
                TriangleRenderNode::INPUTS.as_slice(),
                TriangleRenderNode::OUTPUTS.as_slice(),
            ),
            (
                RaytracerRenderNode::NAME,
                |_| Node::Render(RenderNode::Raytracer(RaytracerRenderNode::default())),
                RaytracerRenderNode::INPUTS.as_slice(),
                RaytracerRenderNode::OUTPUTS.as_slice(),
            ),
            (
                OutputNode::NAME,
                |_| Node::Output(OutputNode::default()),
                OutputNode::INPUTS.as_slice(),
                OutputNode::OUTPUTS.as_slice(),
            ),
            (Self::NUMBER_NAME, |_| Node::Number(0.0), &[], &Self::NUMBER_OUTPUTS),
            (
                Self::STRING_NAME,
                |_| Node::String(String::new()),
                &[],
                &Self::STRING_OUTPUTS,
            ),
            (
                Self::COLOR_NAME,
                |_| Node::Color(Color::default()),
                &[],
                &Self::COLOR_OUTPUTS,
            ),
            (
                Self::VECTOR_NAME,
                |_| Node::Vector(Vector3::default()),
                &[],
                &Self::VECTOR_OUTPUTS,
            ),
            (
                ExpressionNode::NAME,
                |_| Node::Expression(ExpressionNode::new()),
                ExpressionNode::INPUTS.as_slice(),
                ExpressionNode::OUTPUTS.as_slice(),
            ),
        ]
    }

    pub const fn name(&self) -> &str {
        match self {
            Self::Material(MaterialNode::Metal(_)) => MetalNode::NAME,
            Self::Material(MaterialNode::Dielectric(_)) => DielectricNode::NAME,
            Self::Material(MaterialNode::Lambertian(_)) => LambertianNode::NAME,
            Self::Primitive(PrimitiveNode::Sphere(_)) => SphereNode::NAME,
            Self::Collection(_) => CollectionNode::NAME,
            Self::Camera(_) => CameraNode::NAME,
            Self::Render(RenderNode::Triangle(_)) => TriangleRenderNode::NAME,
            Self::Render(RenderNode::Raytracer(_)) => RaytracerRenderNode::NAME,
            Self::Output(_) => OutputNode::NAME,
            Self::Number(_) => Self::NUMBER_NAME,
            Self::String(_) => Self::STRING_NAME,
            Self::Color(_) => Self::COLOR_NAME,
            Self::Vector(_) => Self::VECTOR_NAME,
            Self::Expression(_) => ExpressionNode::NAME,
        }
    }

    pub fn inputs(&self) -> &[u64] {
        match self {
            Self::Material(material) => material.inputs(),
            Self::Primitive(primitive) => primitive.inputs(),
            Self::Collection(collection) => collection.inputs(),
            Self::Camera(camera) => camera.inputs(),
            Self::Render(render) => render.inputs(),
            Self::Output(output) => output.inputs(),
            Self::Number(_) => &[],
            Self::String(_) => &[],
            Self::Color(_) => &[],
            Self::Vector(_) => &[],
            Self::Expression(expression) => expression.inputs(),
        }
    }

    pub fn outputs(&self) -> &[u64] {
        match self {
            Self::Material(material) => material.outputs(),
            Self::Primitive(primitive) => primitive.outputs(),
            Self::Collection(collection) => collection.outputs(),
            Self::Camera(camera) => camera.outputs(),
            Self::Render(render) => render.outputs(),
            Self::Output(output) => output.outputs(),
            Self::Number(_) => &Self::NUMBER_OUTPUTS,
            Self::String(_) => &Self::STRING_OUTPUTS,
            Self::Color(_) => &Self::COLOR_OUTPUTS,
            Self::Vector(_) => &Self::VECTOR_OUTPUTS,
            Self::Expression(expression) => expression.outputs(),
        }
    }

    pub fn disconnect_input(&mut self, input: usize) {
        match self {
            Self::Material(material) => material.disconnect_input(input),
            Self::Primitive(primitive) => primitive.disconnect_input(input),
            Self::Collection(collection) => collection.disconnect_input(input),
            Self::Camera(camera) => camera.disconnect_input(input),
            Self::Render(render) => render.disconnect_input(input),
            Self::Output(output) => output.disconnect_input(input),
            Self::Expression(expression) => expression.disconnect_input(input),
            node => unreachable!("{} node has no inputs", node.name()),
        }
    }

    fn number_out(&self) -> f64 {
        match self {
            Self::Number(value) => *value,
            Self::Expression(expr_node) => expr_node.eval(),
            _ => unreachable!(),
        }
    }

    fn string_out(&self) -> &str {
        match self {
            Self::String(value) => value,
            _ => unreachable!(),
        }
    }

    fn string_in(&mut self) -> &mut String {
        match self {
            Self::Expression(expr_node) => &mut expr_node.text,
            _ => unreachable!(),
        }
    }

    fn as_material_mut(&mut self) -> &mut MaterialNode {
        match self {
            Self::Material(material_node) => material_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), MaterialNode::NAME),
        }
    }

    fn as_primitive_mut(&mut self) -> &mut PrimitiveNode {
        match self {
            Self::Primitive(primitive_node) => primitive_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), PrimitiveNode::NAME),
        }
    }

    fn as_camera_mut(&mut self) -> &mut CameraNode {
        match self {
            Self::Camera(camera_node) => camera_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), CameraNode::NAME),
        }
    }

    fn render_ref(&self) -> Option<&RenderNode> {
        match self {
            Self::Render(render_node) => Some(render_node),
            _ => None,
        }
    }

    fn as_render_mut(&mut self) -> &mut RenderNode {
        match self {
            Self::Render(render_node) => render_node,
            node => panic!("Node `{}` is not an `{}`", node.name(), RenderNode::NAME),
        }
    }

    fn output_ref(&self) -> Option<&OutputNode> {
        match self {
            Self::Output(output_node) => Some(output_node),
            _ => None,
        }
    }

    fn as_expression_mut(&mut self) -> &mut ExpressionNode {
        match self {
            Self::Expression(expr_node) => expr_node,
            node => panic!("Node `{}` is not an `{}`", node.name(), ExpressionNode::NAME),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct OutputNode;

impl OutputNode {
    pub const NAME: &str = "Output";
    pub const INPUTS: [u64; 1] = [NodeFlags::RENDERS.bits()];
    pub const OUTPUTS: [u64; 0] = [];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, _snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => empty_input_view(ui, "Output"),
            _ => unreachable!(),
        }
    }

    pub fn disconnect_input(&mut self, _input: usize) {}
}
