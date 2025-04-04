use eframe::egui_wgpu::RenderState;
use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, Snarl};
use serde::{Deserialize, Serialize};

use crate::node::viewer::empty_input_view;
use crate::node::{Node, NodeFlags};
use crate::types::NodePin;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct RaytracerRenderNode {
    camera: NodePin<Option<NodeId>>,
}

impl RaytracerRenderNode {
    pub const NAME: &str = "Raytracer Render";
    pub const INPUTS: [u64; 1] = [NodeFlags::CAMERA.bits()];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::RENDER_RAYTRACER.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "Camera";

                let remote_value = match &*pin.remotes {
                    [] => None,
                    [remote] => Some(match &snarl[remote.node] {
                        Node::Camera(_) => Some(remote.node),
                        node => unreachable!("{LABEL} input not suppor connection with `{}`", node.name()),
                    }),
                    _ => None,
                };

                if let Some(value) = remote_value {
                    let node = snarl[pin.id.node].as_render_mut().as_raytracer_render_mut();
                    node.camera.set(value);
                }

                empty_input_view(ui, LABEL)
            },
            _ => unreachable!(),
        }
    }

    pub fn disconnect_input(&mut self, input: usize) {
        match input {
            0 => self.camera.reset(),
            _ => unreachable!(),
        }
    }

    pub fn register(&self, _render_state: &RenderState) {}

    pub fn unregister(&self, _render_state: &RenderState) {}
}
