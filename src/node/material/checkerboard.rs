use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, OutPin};
use serde::{Deserialize, Serialize};

use crate::node::NodeFlags;
use crate::node::message::{EventMessage, EventResponse, MessageHandling, SelfNodeMut};
use crate::node::subscribtion::{Event, Subscription};
use crate::node::viewer::{color_input_remote_value, color_input_view};
use crate::types::{Color, NodePin};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckerboardNode {
    pub even: NodePin<Color>,
    pub odd: NodePin<Color>,

    #[serde(skip)]
    subscription: Subscription,
}

impl Default for CheckerboardNode {
    fn default() -> Self {
        Self {
            even: NodePin::new(Color::BLACK),
            odd: NodePin::new(Color::WHITE),
            subscription: Subscription::default(),
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
}

impl MessageHandling for CheckerboardNode {
    fn handle_input_show(mut self_node: SelfNodeMut, pin: &InPin, ui: &mut Ui) -> Option<PinInfo> {
        Some(match pin.id.input {
            0 => {
                const LABEL: &str = "Even";

                let remote_value = color_input_remote_value(pin, self_node.snarl, LABEL);
                let node = self_node.as_material_node_mut().as_checkerboard_mut();

                let old_value = node.even.get();
                let info = color_input_view(ui, LABEL, &mut node.even, remote_value);

                if old_value != node.even.get() {
                    if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
                        caller(self_node)
                    }
                }
                info
            },
            1 => {
                const LABEL: &str = "Odd";

                let remote_value = color_input_remote_value(pin, self_node.snarl, LABEL);
                let node = self_node.as_material_node_mut().as_checkerboard_mut();

                let old_value = node.odd.get();
                let info = color_input_view(ui, LABEL, &mut node.odd, remote_value);

                if old_value != node.odd.get() {
                    if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
                        caller(self_node)
                    }
                }
                info
            },
            _ => unreachable!(),
        })
    }

    fn handle_input_connect(mut self_node: SelfNodeMut, _from: &OutPin, _to: &InPin) {
        let node = self_node.as_material_node_mut().as_checkerboard_mut();
        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_input_disconnect(mut self_node: SelfNodeMut, _from: &OutPin, to: &InPin) {
        let node = self_node.as_material_node_mut().as_checkerboard_mut();
        match to.id.input {
            0 => node.even.reset(),
            1 => node.odd.reset(),
            _ => unreachable!(),
        }

        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_event(mut self_node: SelfNodeMut, event_msg: EventMessage) -> Option<EventResponse> {
        let node = self_node.as_material_node_mut().as_checkerboard_mut();
        node.subscription.handle_event(event_msg)
    }
}
