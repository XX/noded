use egui_snarl::NodeId;
use serde::{Deserialize, Serialize};

pub use self::checkerboard::CheckerboardNode;
pub use self::dielectric::DielectricNode;
pub use self::emissive::EmissiveNode;
pub use self::lambertian::LambertianNode;
pub use self::metal::MetalNode;
use super::message::{CommonNodeMessage, CommonNodeResponse, MessageHandling, SelfNodeMut};

pub mod checkerboard;
pub mod dielectric;
pub mod emissive;
pub mod lambertian;
pub mod metal;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum InputMaterial {
    Internal(MaterialNode),
    External(NodeId),
}

impl Default for InputMaterial {
    fn default() -> Self {
        Self::Internal(MaterialNode::default())
    }
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum MaterialNode {
    Metal(MetalNode),
    Dielectric(DielectricNode),
    Lambertian(LambertianNode),
    Emissive(EmissiveNode),
    Checkerboard(CheckerboardNode),
}

impl Default for MaterialNode {
    fn default() -> Self {
        Self::Lambertian(LambertianNode::default())
    }
}

impl MaterialNode {
    pub const NAME: &str = "Material";

    pub fn name(&self) -> &'static str {
        match self {
            Self::Metal(_) => MetalNode::NAME,
            Self::Dielectric(_) => DielectricNode::NAME,
            Self::Lambertian(_) => LambertianNode::NAME,
            Self::Emissive(_) => EmissiveNode::NAME,
            Self::Checkerboard(_) => CheckerboardNode::NAME,
        }
    }

    pub fn inputs(&self) -> &[u64] {
        match self {
            Self::Metal(metal) => metal.inputs(),
            Self::Dielectric(dielectric) => dielectric.inputs(),
            Self::Lambertian(lambert) => lambert.inputs(),
            Self::Emissive(emissive) => emissive.inputs(),
            Self::Checkerboard(checkerboard) => checkerboard.inputs(),
        }
    }

    pub fn outputs(&self) -> &[u64] {
        match self {
            Self::Metal(metal) => metal.outputs(),
            Self::Dielectric(dielectric) => dielectric.outputs(),
            Self::Lambertian(lambert) => lambert.outputs(),
            Self::Emissive(emissive) => emissive.outputs(),
            Self::Checkerboard(checkerboard) => checkerboard.outputs(),
        }
    }

    pub fn handle_msg(self_node: SelfNodeMut, msg: CommonNodeMessage) -> Option<CommonNodeResponse> {
        match self_node.as_material_node_ref() {
            Self::Metal(_) => MetalNode::handle_msg(self_node, msg),
            Self::Dielectric(_) => DielectricNode::handle_msg(self_node, msg),
            Self::Lambertian(_) => LambertianNode::handle_msg(self_node, msg),
            Self::Emissive(_) => EmissiveNode::handle_msg(self_node, msg),
            Self::Checkerboard(_) => CheckerboardNode::handle_msg(self_node, msg),
        }
    }

    pub fn as_metal_mut(&mut self) -> &mut MetalNode {
        match self {
            Self::Metal(metal) => metal,
            node => panic!("Node `{}` is not a `{}`", node.name(), MetalNode::NAME),
        }
    }

    pub fn as_dielectric_mut(&mut self) -> &mut DielectricNode {
        match self {
            Self::Dielectric(dielectric) => dielectric,
            node => panic!("Node `{}` is not a `{}`", node.name(), DielectricNode::NAME),
        }
    }

    pub fn as_lambert_mut(&mut self) -> &mut LambertianNode {
        match self {
            Self::Lambertian(lambert) => lambert,
            node => panic!("Node `{}` is not a `{}`", node.name(), LambertianNode::NAME),
        }
    }

    pub fn as_emissive_mut(&mut self) -> &mut EmissiveNode {
        match self {
            Self::Emissive(emissive) => emissive,
            node => panic!("Node `{}` is not a `{}`", node.name(), EmissiveNode::NAME),
        }
    }

    pub fn as_checkerboard_mut(&mut self) -> &mut CheckerboardNode {
        match self {
            Self::Checkerboard(checkerboard) => checkerboard,
            node => panic!("Node `{}` is not a `{}`", node.name(), CheckerboardNode::NAME),
        }
    }

    pub fn get_texture_node_id(&self) -> Option<NodeId> {
        match self {
            Self::Metal(metal) => metal.texture.get(),
            Self::Dielectric(_) => None,
            Self::Lambertian(lambert) => lambert.texture.get(),
            Self::Emissive(emissive) => emissive.texture.get(),
            Self::Checkerboard(_) => None,
        }
    }
}
