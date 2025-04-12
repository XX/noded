use bitflags::bitflags;
use eframe::wgpu::naga::FastIndexSet;
use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, Snarl};
use serde::{Deserialize, Serialize};

use self::camera::CameraNode;
use self::collection::CollectionNode;
use self::expression::ExpressionNode;
use self::material::{CheckerboardNode, DielectricNode, EmissiveNode, LambertianNode, MaterialNode, MetalNode};
use self::message::{CommonNodeMessage, CommonNodeResponse, InputMessage, MessageHandling, SelfNodeMut};
use self::primitive::{PrimitiveNode, SphereNode};
use self::render::RenderNode;
use self::render::raytracer::RaytracerRenderNode;
use self::render::triangle::TriangleRenderNode;
use self::scene::SceneNode;
use self::texture::TextureNode;
use self::viewer::{NodeConfig, empty_input_view};
use crate::types::{Color, Vector3};

pub mod camera;
pub mod collection;
pub mod expression;
pub mod material;
pub mod message;
pub mod primitive;
pub mod render;
pub mod scene;
pub mod subscribtion;
pub mod texture;
pub mod viewer;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct NodeFlags: u64 {
        const MATERIAL_METAL = 0b00000001;
        const MATERIAL_DIELECTRIC = Self::MATERIAL_METAL.bits() << 1;
        const MATERIAL_LAMBERT = Self::MATERIAL_DIELECTRIC.bits() << 1;
        const MATERIAL_EMISSIVE = Self::MATERIAL_LAMBERT.bits() << 1;
        const MATERIAL_CHECKERBOARD = Self::MATERIAL_EMISSIVE.bits() << 1;
        const MATERIALS = Self::MATERIAL_METAL.bits() | Self::MATERIAL_DIELECTRIC.bits() | Self::MATERIAL_LAMBERT.bits() | Self::MATERIAL_EMISSIVE.bits() | Self::MATERIAL_CHECKERBOARD.bits();

        const TEXTURE = Self::MATERIAL_CHECKERBOARD.bits() << 1;

        const PRIMITIVE_SPHERE = Self::TEXTURE.bits() << 1;
        const PRIMITIVES = Self::PRIMITIVE_SPHERE.bits();

        const COLLECTION = Self::PRIMITIVE_SPHERE.bits() << 1;
        const CAMERA = Self::COLLECTION.bits() << 1;

        const SCENE = Self::CAMERA.bits() << 1;

        const RENDER_TRIANGLE = Self::SCENE.bits() << 1;
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
    Texture(TextureNode),
    Primitive(PrimitiveNode),
    Collection(CollectionNode),
    Camera(CameraNode),
    Scene(SceneNode),
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
                LambertianNode::NAME,
                |_| Node::Material(MaterialNode::Lambertian(LambertianNode::default())),
                LambertianNode::INPUTS.as_slice(),
                LambertianNode::OUTPUTS.as_slice(),
            ),
            (
                EmissiveNode::NAME,
                |_| Node::Material(MaterialNode::Emissive(EmissiveNode::default())),
                EmissiveNode::INPUTS.as_slice(),
                EmissiveNode::OUTPUTS.as_slice(),
            ),
            (
                CheckerboardNode::NAME,
                |_| Node::Material(MaterialNode::Checkerboard(CheckerboardNode::default())),
                CheckerboardNode::INPUTS.as_slice(),
                CheckerboardNode::OUTPUTS.as_slice(),
            ),
            (
                TextureNode::NAME,
                |_| Node::Texture(TextureNode::default()),
                TextureNode::INPUTS.as_slice(),
                TextureNode::OUTPUTS.as_slice(),
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
                &[CollectionNode::INPUT],
                CollectionNode::OUTPUTS.as_slice(),
            ),
            (
                CameraNode::NAME,
                |_| Node::Camera(CameraNode::default()),
                CameraNode::INPUTS.as_slice(),
                CameraNode::OUTPUTS.as_slice(),
            ),
            (
                SceneNode::NAME,
                |_| Node::Scene(SceneNode::default()),
                SceneNode::INPUTS.as_slice(),
                SceneNode::OUTPUTS.as_slice(),
            ),
            (
                TriangleRenderNode::NAME,
                |_| Node::Render(RenderNode::Triangle(TriangleRenderNode::default())),
                TriangleRenderNode::INPUTS.as_slice(),
                TriangleRenderNode::OUTPUTS.as_slice(),
            ),
            (
                RaytracerRenderNode::NAME,
                |config| {
                    Node::Render(RenderNode::Raytracer(RaytracerRenderNode::new(
                        config.max_viewport_resolution,
                    )))
                },
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
            Self::Material(MaterialNode::Emissive(_)) => EmissiveNode::NAME,
            Self::Material(MaterialNode::Checkerboard(_)) => CheckerboardNode::NAME,
            Self::Texture(_) => TextureNode::NAME,
            Self::Primitive(PrimitiveNode::Sphere(_)) => SphereNode::NAME,
            Self::Collection(_) => CollectionNode::NAME,
            Self::Camera(_) => CameraNode::NAME,
            Self::Scene(_) => SceneNode::NAME,
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
            Self::Texture(texture) => texture.inputs(),
            Self::Primitive(primitive) => primitive.inputs(),
            Self::Collection(collection) => collection.inputs(),
            Self::Camera(camera) => camera.inputs(),
            Self::Scene(scene) => scene.inputs(),
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
            Self::Texture(texture) => texture.outputs(),
            Self::Primitive(primitive) => primitive.outputs(),
            Self::Collection(collection) => collection.outputs(),
            Self::Camera(camera) => camera.outputs(),
            Self::Scene(scene) => scene.outputs(),
            Self::Render(render) => render.outputs(),
            Self::Output(output) => output.outputs(),
            Self::Number(_) => &Self::NUMBER_OUTPUTS,
            Self::String(_) => &Self::STRING_OUTPUTS,
            Self::Color(_) => &Self::COLOR_OUTPUTS,
            Self::Vector(_) => &Self::VECTOR_OUTPUTS,
            Self::Expression(expression) => expression.outputs(),
        }
    }

    pub fn send_msg<'a>(
        self_id: NodeId,
        snarl: &mut Snarl<Node>,
        msg: impl Into<CommonNodeMessage<'a>>,
    ) -> Option<CommonNodeResponse> {
        let self_node = SelfNodeMut::new(self_id, snarl);
        Self::handle_msg(self_node, msg)
    }

    pub fn handle_msg<'a>(self_node: SelfNodeMut, msg: impl Into<CommonNodeMessage<'a>>) -> Option<CommonNodeResponse> {
        let msg = msg.into();

        match self_node.node_ref() {
            Self::Material(_) => MaterialNode::handle_msg(self_node, msg),
            Self::Texture(_) => TextureNode::handle_msg(self_node, msg),
            Self::Primitive(_) => PrimitiveNode::handle_msg(self_node, msg),
            Self::Collection(_) => CollectionNode::handle_msg(self_node, msg),
            Self::Camera(_) => CameraNode::handle_msg(self_node, msg),
            Self::Scene(_) => SceneNode::handle_msg(self_node, msg),
            Self::Render(_) => RenderNode::handle_msg(self_node, msg),
            Self::Output(_) => OutputNode::handle_msg(self_node, msg),
            Self::Expression(_) => ExpressionNode::handle_msg(self_node, msg),
            _ => None,
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

    fn as_material_node_ref(&self) -> &MaterialNode {
        match self {
            Self::Material(material_node) => material_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), MaterialNode::NAME),
        }
    }

    fn as_material_node_mut(&mut self) -> &mut MaterialNode {
        match self {
            Self::Material(material_node) => material_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), MaterialNode::NAME),
        }
    }

    fn as_texture_node_mut(&mut self) -> &mut TextureNode {
        match self {
            Self::Texture(texture_node) => texture_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), TextureNode::NAME),
        }
    }

    fn primitive_node_ref(&self) -> Option<&PrimitiveNode> {
        match self {
            Self::Primitive(primitive_node) => Some(primitive_node),
            _ => None,
        }
    }

    fn as_primitive_node_ref(&self) -> &PrimitiveNode {
        self.primitive_node_ref()
            .unwrap_or_else(|| panic!("Node `{}` is not a `{}`", self.name(), PrimitiveNode::NAME))
    }

    fn as_primitive_node_mut(&mut self) -> &mut PrimitiveNode {
        match self {
            Self::Primitive(primitive_node) => primitive_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), PrimitiveNode::NAME),
        }
    }

    fn collection_node_ref(&self) -> Option<&CollectionNode> {
        match self {
            Self::Collection(collection_node) => Some(collection_node),
            _ => None,
        }
    }

    pub fn as_collection_node_ref(&self) -> &CollectionNode {
        self.collection_node_ref()
            .unwrap_or_else(|| panic!("Node `{}` is not a `{}`", self.name(), CollectionNode::NAME))
    }

    fn as_collection_node_mut(&mut self) -> &mut CollectionNode {
        match self {
            Self::Collection(collection_node) => collection_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), CollectionNode::NAME),
        }
    }

    fn camera_node_ref(&self) -> Option<&CameraNode> {
        match self {
            Self::Camera(camera_node) => Some(camera_node),
            _ => None,
        }
    }

    fn camera_node_mut(&mut self) -> Option<&mut CameraNode> {
        match self {
            Self::Camera(camera_node) => Some(camera_node),
            _ => None,
        }
    }

    fn as_camera_node(&self) -> &CameraNode {
        match self {
            Self::Camera(camera_node) => camera_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), CameraNode::NAME),
        }
    }

    fn as_camera_node_mut(&mut self) -> &mut CameraNode {
        match self {
            Self::Camera(camera_node) => camera_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), CameraNode::NAME),
        }
    }

    fn as_scene_node_ref(&self) -> &SceneNode {
        match self {
            Self::Scene(scene_node) => scene_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), SceneNode::NAME),
        }
    }

    fn as_scene_node_mut(&mut self) -> &mut SceneNode {
        match self {
            Self::Scene(scene_node) => scene_node,
            node => panic!("Node `{}` is not a `{}`", node.name(), SceneNode::NAME),
        }
    }

    fn render_node_ref(&self) -> Option<&RenderNode> {
        match self {
            Self::Render(render_node) => Some(render_node),
            _ => None,
        }
    }

    fn as_render_node_ref(&self) -> &RenderNode {
        self.render_node_ref()
            .unwrap_or_else(|| panic!("Node `{}` is not a `{}`", self.name(), RenderNode::NAME))
    }

    fn as_render_node_mut(&mut self) -> &mut RenderNode {
        match self {
            Self::Render(render_node) => render_node,
            node => panic!("Node `{}` is not an `{}`", node.name(), RenderNode::NAME),
        }
    }

    fn output_node_ref(&self) -> Option<&OutputNode> {
        match self {
            Self::Output(output_node) => Some(output_node),
            _ => None,
        }
    }

    fn as_expression_node_mut(&mut self) -> &mut ExpressionNode {
        match self {
            Self::Expression(expr_node) => expr_node,
            node => panic!("Node `{}` is not an `{}`", node.name(), ExpressionNode::NAME),
        }
    }
}

pub fn collect_for_node(
    node_id: Option<NodeId>,
    predicate: &dyn Fn(&Node) -> bool,
    destination: &mut FastIndexSet<NodeId>,
    snarl: &mut Snarl<Node>,
) {
    if let Some(node_id) = node_id {
        let self_node = SelfNodeMut::new(node_id, snarl);
        let need_insert = predicate(self_node.node_ref());

        Node::handle_msg(
            self_node,
            CommonNodeMessage::Input(InputMessage::CollectIds { predicate, destination }),
        );
        if need_insert {
            destination.insert(node_id);
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
}

impl MessageHandling for OutputNode {
    fn handle_input_show(_self_node: SelfNodeMut, pin: &InPin, ui: &mut Ui) -> Option<PinInfo> {
        Some(match pin.id.input {
            0 => empty_input_view(ui, "Output"),
            _ => unreachable!(),
        })
    }
}
