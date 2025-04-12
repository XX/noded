use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin};
use serde::{Deserialize, Serialize};

use crate::node::message::{EventMessage, EventResponse, MessageHandling, SelfNodeMut};
use crate::node::subscribtion::{Event, Subscription};
use crate::node::viewer::{
    color_input_remote_value, color_input_view, empty_input_view, number_input_remote_value, number_input_view,
};
use crate::node::{Node, NodeFlags, collect_for_node};
use crate::types::{Color, NodePin};

#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MetalNode {
    pub albedo: NodePin<Color>,
    pub fuzz: NodePin<f64>,
    pub texture: NodePin<Option<NodeId>>,

    #[serde(skip)]
    subscription: Subscription,
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
}

impl MessageHandling for MetalNode {
    fn handle_input_show(mut self_node: SelfNodeMut, pin: &InPin, ui: &mut Ui) -> Option<PinInfo> {
        Some(match pin.id.input {
            0 => {
                const LABEL: &str = "Albedo";

                let remote_value = color_input_remote_value(pin, self_node.snarl, LABEL);
                let node = self_node.as_material_node_mut().as_metal_mut();

                let old_value = node.albedo.get();
                let info = color_input_view(ui, LABEL, &mut node.albedo, remote_value);

                if old_value != node.albedo.get() {
                    if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
                        caller(self_node)
                    }
                }
                info
            },
            1 => {
                const LABEL: &str = "Fuzz";

                let remote_value = number_input_remote_value(pin, self_node.snarl, LABEL);
                let node = self_node.as_material_node_mut().as_metal_mut();

                let old_value = node.fuzz.get();
                let info = number_input_view(ui, LABEL, &mut node.fuzz, remote_value);

                if old_value != node.fuzz.get() {
                    if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
                        caller(self_node)
                    }
                }
                info
            },
            2 => {
                const LABEL: &str = "Texture";

                let remote_value = match &*pin.remotes {
                    [] => None,
                    [remote] => Some(match &self_node.snarl[remote.node] {
                        Node::Texture(_) => Some(remote.node),
                        node => unreachable!("{LABEL} input not suppor connection with `{}`", node.name()),
                    }),
                    _ => None,
                };

                if let Some(value) = remote_value {
                    let node = self_node.as_material_node_mut().as_metal_mut();
                    node.texture.set(value);
                }

                empty_input_view(ui, LABEL)
            },
            _ => unreachable!(),
        })
    }

    fn handle_input_connect(mut self_node: SelfNodeMut, _from: &OutPin, _to: &InPin) {
        let node = self_node.as_material_node_mut().as_metal_mut();
        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_input_disconnect(mut self_node: SelfNodeMut, _from: &OutPin, to: &InPin) {
        let node = self_node.as_material_node_mut().as_metal_mut();
        match to.id.input {
            0 => node.albedo.reset(),
            1 => node.fuzz.reset(),
            2 => node.texture.reset(),
            _ => unreachable!(),
        }

        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_input_collect_ids(
        self_node: SelfNodeMut,
        predicate: &dyn Fn(&Node) -> bool,
        destination: &mut eframe::wgpu::naga::FastIndexSet<NodeId>,
    ) {
        collect_for_node(
            self_node.as_material_node_ref().get_texture_node_id(),
            predicate,
            destination,
            self_node.snarl,
        );
    }

    fn handle_event(mut self_node: SelfNodeMut, event_msg: EventMessage) -> Option<EventResponse> {
        let node = self_node.as_material_node_mut().as_metal_mut();
        node.subscription.handle_event(event_msg)
    }
}
