use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, Snarl};
use serde::{Deserialize, Serialize};

use crate::node::viewer::{
    empty_input_view, number_input_remote_value, number_input_view, vector_input_remote_value, vector_input_view,
};
use crate::node::{Node, NodeFlags};
use crate::types::{NodePin, Point3};

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CameraNode {
    position: NodePin<Point3>,
    yaw: NodePin<f64>,
    pitch: NodePin<f64>,
    vfov_degrees: NodePin<f64>,
    aperture: NodePin<f64>,
    focus_distance: NodePin<f64>,
    scene: NodePin<Vec<NodeId>>,
}

impl CameraNode {
    pub const NAME: &str = "Camera";
    pub const INPUTS: [u64; 7] = [
        NodeFlags::TYPICAL_VECTOR_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::PRIMITIVES.bits() | NodeFlags::COLLECTION.bits(),
    ];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::CAMERA.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "Position";

                let remote_value = vector_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_mut();
                vector_input_view(ui, LABEL, &mut node.position, remote_value)
            },
            1 => {
                const LABEL: &str = "Yaw";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_mut();
                number_input_view(ui, LABEL, &mut node.yaw, remote_value)
            },
            2 => {
                const LABEL: &str = "Pitch";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_mut();
                number_input_view(ui, LABEL, &mut node.pitch, remote_value)
            },
            3 => {
                const LABEL: &str = "VFOV";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_mut();
                number_input_view(ui, LABEL, &mut node.vfov_degrees, remote_value)
            },
            4 => {
                const LABEL: &str = "Aperture";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_mut();
                number_input_view(ui, LABEL, &mut node.aperture, remote_value)
            },
            5 => {
                const LABEL: &str = "Focus Distance";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_camera_mut();
                number_input_view(ui, LABEL, &mut node.focus_distance, remote_value)
            },
            6 => {
                const LABEL: &str = "Scene";

                let remote_value = match &*pin.remotes {
                    [] => None,
                    [remote] => Some(match &snarl[remote.node] {
                        Node::Primitive(_) => vec![remote.node],
                        Node::Collection(collection) => collection.cloned_nodes(),
                        node => unreachable!("{LABEL} input not suppor connection with `{}`", node.name()),
                    }),
                    _ => None,
                };

                if let Some(value) = remote_value {
                    let Node::Camera(node) = &mut snarl[pin.id.node] else {
                        panic!()
                    };
                    node.scene.set(value);
                }

                empty_input_view(ui, LABEL)
            },
            _ => unreachable!(),
        }
    }

    pub fn disconnect_input(&mut self, input: usize) {
        match input {
            0 => self.position.reset(),
            1 => self.yaw.reset(),
            2 => self.pitch.reset(),
            3 => self.vfov_degrees.reset(),
            4 => self.aperture.reset(),
            5 => self.focus_distance.reset(),
            6 => self.scene.reset(),
            _ => unreachable!(),
        }
    }
}
