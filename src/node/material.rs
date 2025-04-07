use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use serde::{Deserialize, Serialize};

use super::viewer::{color_input_view, number_input_view};
use super::{Node, NodeFlags};
use crate::node::viewer::{color_input_remote_value, empty_input_view, number_input_remote_value};
use crate::types::{Color, NodePin};

#[derive(Clone, Serialize, Deserialize)]
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

    pub fn connect_input(&mut self, from: &OutPin, to: &InPin) {
        match self {
            Self::Metal(metal) => metal.connect_input(from, to),
            Self::Dielectric(dielectric) => dielectric.connect_input(from, to),
            Self::Lambertian(lambert) => lambert.connect_input(from, to),
            Self::Emissive(emissive) => emissive.connect_input(from, to),
            Self::Checkerboard(checkerboard) => checkerboard.connect_input(from, to),
        }
    }

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match self {
            Self::Metal(metal) => metal.disconnect_input(input_pin),
            Self::Dielectric(dielectric) => dielectric.disconnect_input(input_pin),
            Self::Lambertian(lambert) => lambert.disconnect_input(input_pin),
            Self::Emissive(emissive) => emissive.disconnect_input(input_pin),
            Self::Checkerboard(checkerboard) => checkerboard.disconnect_input(input_pin),
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
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct MetalNode {
    albedo: NodePin<Color>,
    fuzz: NodePin<f64>,
    texture: NodePin<Option<NodeId>>,
}

impl MetalNode {
    pub const NAME: &str = "Metal Material";
    pub const INPUTS: [u64; 3] = [
        NodeFlags::TYPICAL_VECTOR_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TEXTURE.bits(),
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
                let node = snarl[pin.id.node].as_material_node_mut().as_metal_mut();
                color_input_view(ui, LABEL, &mut node.albedo, remote_value)
            },
            1 => {
                const LABEL: &str = "Fuzz";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_material_node_mut().as_metal_mut();
                number_input_view(ui, LABEL, &mut node.fuzz, remote_value)
            },
            2 => {
                const LABEL: &str = "Texture";
                empty_input_view(ui, LABEL)
            },
            _ => unreachable!(),
        }
    }

    pub fn connect_input(&mut self, _from: &OutPin, _to: &InPin) {}

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match input_pin.id.input {
            0 => self.albedo.reset(),
            1 => self.fuzz.reset(),
            2 => self.texture.reset(),
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
    pub const INPUTS: [u64; 1] = [NodeFlags::TYPICAL_NUMBER_INPUT.bits()];
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
                let node = snarl[pin.id.node].as_material_node_mut().as_dielectric_mut();
                number_input_view(ui, LABEL, &mut node.ior, remote_value)
            },
            _ => unreachable!(),
        }
    }

    pub fn connect_input(&mut self, _from: &OutPin, _to: &InPin) {}

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match input_pin.id.input {
            0 => self.ior.reset(),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LambertianNode {
    albedo: NodePin<Color>,
    texture: NodePin<Option<NodeId>>,
}

impl Default for LambertianNode {
    fn default() -> Self {
        Self {
            albedo: NodePin::new(Color::LIGHT_GRAY),
            texture: NodePin::default(),
        }
    }
}

impl LambertianNode {
    pub const NAME: &str = "Lambertian Material";
    pub const INPUTS: [u64; 2] = [NodeFlags::TYPICAL_VECTOR_INPUT.bits(), NodeFlags::TEXTURE.bits()];
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
                let node = snarl[pin.id.node].as_material_node_mut().as_lambert_mut();
                color_input_view(ui, LABEL, &mut node.albedo, remote_value)
            },
            1 => {
                const LABEL: &str = "Texture";
                empty_input_view(ui, LABEL)
            },
            _ => unreachable!(),
        }
    }

    pub fn connect_input(&mut self, _from: &OutPin, _to: &InPin) {}

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match input_pin.id.input {
            0 => self.albedo.reset(),
            1 => self.texture.reset(),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct EmissiveNode {
    emit: NodePin<Color>,
    texture: NodePin<Option<NodeId>>,
}

impl EmissiveNode {
    pub const NAME: &str = "Emissive Material";
    pub const INPUTS: [u64; 2] = [NodeFlags::TYPICAL_VECTOR_INPUT.bits(), NodeFlags::TEXTURE.bits()];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::MATERIAL_EMISSIVE.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "Emit";

                let remote_value = color_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_material_node_mut().as_emissive_mut();
                color_input_view(ui, LABEL, &mut node.emit, remote_value)
            },
            1 => {
                const LABEL: &str = "Texture";
                empty_input_view(ui, LABEL)
            },
            _ => unreachable!(),
        }
    }

    pub fn connect_input(&mut self, _from: &OutPin, _to: &InPin) {}

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match input_pin.id.input {
            0 => self.emit.reset(),
            1 => self.texture.reset(),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CheckerboardNode {
    even: NodePin<Color>,
    odd: NodePin<Color>,
}

impl Default for CheckerboardNode {
    fn default() -> Self {
        Self {
            even: NodePin::new(Color::BLACK),
            odd: NodePin::new(Color::WHITE),
        }
    }
}

impl CheckerboardNode {
    pub const NAME: &str = "Checkerboard Material";
    pub const INPUTS: [u64; 2] = [
        NodeFlags::TYPICAL_VECTOR_INPUT.bits(),
        NodeFlags::TYPICAL_VECTOR_INPUT.bits(),
    ];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::MATERIAL_CHECKERBOARD.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "Even";

                let remote_value = color_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_material_node_mut().as_checkerboard_mut();
                color_input_view(ui, LABEL, &mut node.even, remote_value)
            },
            1 => {
                const LABEL: &str = "Odd";

                let remote_value = color_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_material_node_mut().as_checkerboard_mut();
                color_input_view(ui, LABEL, &mut node.odd, remote_value)
            },
            _ => unreachable!(),
        }
    }

    pub fn connect_input(&mut self, _from: &OutPin, _to: &InPin) {}

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match input_pin.id.input {
            0 => self.even.reset(),
            1 => self.odd.reset(),
            _ => unreachable!(),
        }
    }
}
