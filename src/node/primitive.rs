use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, Snarl};
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

    pub fn as_sphere(&self) -> &SphereNode {
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
        NodeFlags::VECTOR.bits() | NodeFlags::COLOR.bits() | NodeFlags::NUMBER.bits() | NodeFlags::EXPRESSION.bits(),
        NodeFlags::NUMBER.bits() | NodeFlags::EXPRESSION.bits(),
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
                let Node::Primitive(PrimitiveNode::Sphere(node)) = &mut snarl[pin.id.node] else {
                    panic!()
                };
                vector_input_view(ui, LABEL, &mut node.center, remote_value)
            },
            1 => {
                const LABEL: &str = "Radius";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let Node::Primitive(PrimitiveNode::Sphere(node)) = &mut snarl[pin.id.node] else {
                    panic!()
                };
                number_input_view(ui, LABEL, &mut node.radius, remote_value)
            },
            2 => {
                const LABEL: &str = "Material";

                let remote_value = material_input_remote_value(pin, snarl, LABEL);
                let Node::Primitive(PrimitiveNode::Sphere(node)) = &mut snarl[pin.id.node] else {
                    panic!()
                };
                material_input_view(ui, LABEL, &mut node.material, remote_value)
            },
            _ => unreachable!(),
        }
    }
}
