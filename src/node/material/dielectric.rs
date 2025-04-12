use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, OutPin};
use serde::{Deserialize, Serialize};

use crate::node::NodeFlags;
use crate::node::message::{EventMessage, EventResponse, MessageHandling, SelfNodeMut};
use crate::node::subscribtion::{Event, Subscription};
use crate::node::viewer::{number_input_remote_value, number_input_view};
use crate::types::NodePin;

#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DielectricNode {
    pub ior: NodePin<f64>,

    #[serde(skip)]
    subscription: Subscription,
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
}

impl MessageHandling for DielectricNode {
    fn handle_input_show(mut self_node: SelfNodeMut, pin: &InPin, ui: &mut Ui) -> Option<PinInfo> {
        Some(match pin.id.input {
            0 => {
                const LABEL: &str = "IOR";

                let remote_value = number_input_remote_value(pin, self_node.snarl, LABEL);
                let node = self_node.as_material_node_mut().as_dielectric_mut();

                let old_value = node.ior.get();
                let info = number_input_view(ui, LABEL, &mut node.ior, remote_value);

                if old_value != node.ior.get() {
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
        let node = self_node.as_material_node_mut().as_dielectric_mut();
        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_input_disconnect(mut self_node: SelfNodeMut, _from: &OutPin, to: &InPin) {
        let node = self_node.as_material_node_mut().as_dielectric_mut();
        match to.id.input {
            0 => node.ior.reset(),
            _ => unreachable!(),
        }

        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_event(mut self_node: SelfNodeMut, event_msg: EventMessage) -> Option<EventResponse> {
        let node = self_node.as_material_node_mut().as_dielectric_mut();
        node.subscription.handle_event(event_msg)
    }
}
