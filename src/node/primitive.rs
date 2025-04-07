use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, OutPin, Snarl};
use serde::{Deserialize, Serialize};

use super::NodeFlags;
use super::material::MaterialNode;
use crate::node::Node;
use crate::node::viewer::{
    material_input_remote_value, material_input_view, number_input_remote_value, number_input_view,
    vector_input_remote_value, vector_input_view,
};
use crate::types::{NodePin, Vector3};

#[derive(Clone, Serialize, Deserialize)]
pub enum PrimitiveNode {
    Sphere(SphereNode),
}

impl PrimitiveNode {
    pub const NAME: &str = "Primitive";

    pub fn inputs(&self) -> &[u64] {
        match self {
            Self::Sphere(sphere) => sphere.inputs(),
        }
    }

    pub fn outputs(&self) -> &[u64] {
        match self {
            Self::Sphere(sphere) => sphere.outputs(),
        }
    }

    pub fn connect_input(&mut self, _from: &OutPin, _to: &InPin) {}

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match self {
            Self::Sphere(sphere) => sphere.disconnect_input(input_pin),
        }
    }

    pub fn as_sphere_mut(&mut self) -> &mut SphereNode {
        match self {
            Self::Sphere(sphere) => sphere,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SphereNode {
    pub center: NodePin<Vector3>,
    pub radius: NodePin<f64>,
    pub material: NodePin<MaterialNode>,
}

impl Default for SphereNode {
    fn default() -> Self {
        Self {
            center: Default::default(),
            radius: NodePin::new(1.0),
            material: Default::default(),
        }
    }
}

impl SphereNode {
    pub const NAME: &str = "Sphere Primitive";
    pub const INPUTS: [u64; 3] = [
        NodeFlags::TYPICAL_VECTOR_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::MATERIALS.bits(),
    ];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::PRIMITIVE_SPHERE.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "Center";

                let remote_value = vector_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_primitive_node_mut().as_sphere_mut();
                vector_input_view(ui, LABEL, &mut node.center, remote_value)
            },
            1 => {
                const LABEL: &str = "Radius";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_primitive_node_mut().as_sphere_mut();
                number_input_view(ui, LABEL, &mut node.radius, remote_value)
            },
            2 => {
                const LABEL: &str = "Material";

                let remote_value = material_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_primitive_node_mut().as_sphere_mut();
                material_input_view(ui, LABEL, &mut node.material, remote_value)
            },
            _ => unreachable!(),
        }
    }

    pub fn connect_input(&mut self, _input_pin: &InPin) {}

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match input_pin.id.input {
            0 => self.center.reset(),
            1 => self.radius.reset(),
            2 => self.material.reset(),
            _ => unreachable!(),
        }
    }
}
