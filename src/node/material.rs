use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, Snarl};
use serde::{Deserialize, Serialize};

use super::viewer::{color_input_view, number_input_view};
use super::{Node, NodeFlags};
use crate::node::viewer::{color_input_remote_value, number_input_remote_value};
use crate::types::{Color, NodePin};

#[derive(Clone, Serialize, Deserialize)]
pub enum MaterialNode {
    Metal(MetalNode),
    Dielectric(DielectricNode),
    Lambert(LambertNode),
}

impl Default for MaterialNode {
    fn default() -> Self {
        Self::Lambert(LambertNode::default())
    }
}

impl MaterialNode {
    pub const NAME: &str = "Material";

    pub fn name(&self) -> &'static str {
        match self {
            Self::Metal(_) => MetalNode::NAME,
            Self::Dielectric(_) => DielectricNode::NAME,
            Self::Lambert(_) => LambertNode::NAME,
        }
    }

    pub fn inputs(&self) -> &[u64] {
        match self {
            Self::Metal(metal) => metal.inputs(),
            Self::Dielectric(dielectric) => dielectric.inputs(),
            Self::Lambert(lambert) => lambert.inputs(),
        }
    }

    pub fn outputs(&self) -> &[u64] {
        match self {
            Self::Metal(metal) => metal.outputs(),
            Self::Dielectric(dielectric) => dielectric.outputs(),
            Self::Lambert(lambert) => lambert.outputs(),
        }
    }

    pub fn as_metal_node(&mut self) -> &mut MetalNode {
        match self {
            Self::Metal(metal) => metal,
            node => panic!("Node `{}` is not a `{}`", node.name(), MetalNode::NAME),
        }
    }

    pub fn as_dielectric_node(&mut self) -> &mut DielectricNode {
        match self {
            Self::Dielectric(dielectric) => dielectric,
            node => panic!("Node `{}` is not a `{}`", node.name(), DielectricNode::NAME),
        }
    }

    pub fn as_lambert_node(&mut self) -> &mut LambertNode {
        match self {
            Self::Lambert(lambert) => lambert,
            node => panic!("Node `{}` is not a `{}`", node.name(), LambertNode::NAME),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct MetalNode {
    albedo: NodePin<Color>,
    roughness: NodePin<f64>,
}

impl MetalNode {
    pub const NAME: &str = "Metal Material";
    pub const INPUTS: [u64; 2] = [
        NodeFlags::COLOR.bits() | NodeFlags::VECTOR.bits() | NodeFlags::NUMBER.bits() | NodeFlags::EXPRESSION.bits(),
        NodeFlags::NUMBER.bits() | NodeFlags::EXPRESSION.bits(),
    ];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::MATERIAL_METAL.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "Albedo";

                let remote_value = color_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_material_node().as_metal_node();
                color_input_view(ui, LABEL, &mut node.albedo, remote_value)
            },
            1 => {
                const LABEL: &str = "Roughness";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_material_node().as_metal_node();
                number_input_view(ui, LABEL, &mut node.roughness, remote_value)
            },
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DielectricNode {
    pub ior: NodePin<f64>,
}

impl DielectricNode {
    pub const NAME: &str = "Dielectric Material";
    pub const INPUTS: [u64; 1] = [NodeFlags::NUMBER.bits() | NodeFlags::EXPRESSION.bits()];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::MATERIAL_DIELECTRIC.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "IOR";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_material_node().as_dielectric_node();
                number_input_view(ui, LABEL, &mut node.ior, remote_value)
            },
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LambertNode {
    albedo: NodePin<Color>,
}

impl Default for LambertNode {
    fn default() -> Self {
        Self {
            albedo: NodePin::new(Color::LIGHT_GRAY),
        }
    }
}

impl LambertNode {
    pub const NAME: &str = "Lambert Material";
    pub const INPUTS: [u64; 1] = [NodeFlags::COLOR.bits()
        | NodeFlags::VECTOR.bits()
        | NodeFlags::NUMBER.bits()
        | NodeFlags::EXPRESSION.bits()];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::MATERIAL_LAMBERT.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "Albedo";

                let remote_value = color_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_material_node().as_lambert_node();
                color_input_view(ui, LABEL, &mut node.albedo, remote_value)
            },
            _ => unreachable!(),
        }
    }
}
