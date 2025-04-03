use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId};
use serde::{Deserialize, Serialize};

use super::NodeFlags;
use super::viewer::empty_input_view;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct CollectionNode {
    nodes: Vec<NodeId>,
    inputs: Vec<u64>,
}

impl CollectionNode {
    pub const NAME: &str = "Collection";
    pub const OUTPUTS: [u64; 1] = [NodeFlags::COLLECTION.bits()];

    pub fn add(&mut self, node: NodeId) {
        self.nodes.push(node);
        self.inputs.push(NodeFlags::ALL.bits());
    }

    pub fn nodes(&self) -> &[NodeId] {
        &self.nodes
    }

    pub fn cloned_nodes(&self) -> Vec<NodeId> {
        self.nodes.clone()
    }

    pub fn inputs(&self) -> &[u64] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(&self, pin: &InPin, ui: &mut Ui) -> PinInfo {
        empty_input_view(ui, &(pin.id.input + 1).to_string())
    }

    pub fn disconnect_input(&mut self, input: usize) {
        self.nodes.remove(input);
        self.inputs.remove(input);
    }
}
