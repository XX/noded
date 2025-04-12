use egui::Ui;
use egui_snarl::ui::{PinInfo, WireStyle};
use egui_snarl::{InPin, OutPin};
use serde::{Deserialize, Serialize};

use super::NodeFlags;
use super::message::{EventMessage, EventResponse, MessageHandling, SelfNodeMut};
use super::subscribtion::Subscription;
use super::viewer::STRING_COLOR;
use crate::node::subscribtion::Event;
use crate::node::viewer::{number_input_remote_value, number_input_view};
use crate::types::NodePin;

#[derive(Clone, Serialize, Deserialize)]
pub struct TextureNode {
    pub path: String,
    pub scale: NodePin<f64>,

    #[serde(skip)]
    subscription: Subscription,
}

impl Default for TextureNode {
    fn default() -> Self {
        Self {
            path: String::new(),
            scale: NodePin::new(1.0),
            subscription: Subscription::default(),
        }
    }
}

impl TextureNode {
    pub const NAME: &str = "Texture";
    pub const INPUTS: [u64; 1] = [NodeFlags::TYPICAL_NUMBER_INPUT.bits()];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::TEXTURE.bits() | NodeFlags::STRING.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_output(mut self_node: SelfNodeMut, _pin: &OutPin, ui: &mut Ui) -> PinInfo {
        let node = self_node.as_texture_node_mut();
        let old_value = node.path.clone();
        let edit: egui::TextEdit<'_> = egui::TextEdit::singleline(&mut node.path)
            .clip_text(false)
            .desired_width(0.0)
            .margin(ui.spacing().item_spacing);
        ui.horizontal(|ui| {
            ui.add(edit);
            ui.label("Path");
        });

        if old_value != node.path {
            if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
                caller(self_node);
            }
        }

        PinInfo::circle()
            .with_fill(STRING_COLOR)
            .with_wire_style(WireStyle::AxisAligned { corner_radius: 10.0 })
    }
}

impl MessageHandling for TextureNode {
    fn handle_input_show(mut self_node: SelfNodeMut, pin: &InPin, ui: &mut Ui) -> Option<PinInfo> {
        Some(match pin.id.input {
            0 => {
                const LABEL: &str = "Scale";

                let remote_value = number_input_remote_value(pin, self_node.snarl, LABEL);
                let node = self_node.as_texture_node_mut();

                let old_value = node.scale.get();
                let info = number_input_view(ui, LABEL, &mut node.scale, remote_value);

                if old_value != node.scale.get() {
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
        let node = self_node.as_texture_node_mut();
        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_input_disconnect(mut self_node: SelfNodeMut, _from: &OutPin, to: &InPin) {
        let node = self_node.as_texture_node_mut();
        match to.id.input {
            0 => node.scale.reset(),
            _ => unreachable!(),
        }

        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_event(mut self_node: SelfNodeMut, event_msg: EventMessage) -> Option<EventResponse> {
        let node = self_node.as_texture_node_mut();
        node.subscription.handle_event(event_msg)
    }
}
