use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use serde::{Deserialize, Serialize};

use super::viewer::empty_input_view;
use super::{Node, NodeFlags};

#[derive(Clone, Serialize, Deserialize)]
pub struct CollectionNode {
    nodes: Vec<NodeId>,
    inputs: Vec<u64>,
}

impl Default for CollectionNode {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            inputs: vec![NodeFlags::ALL.bits()],
        }
    }
}

impl CollectionNode {
    pub const NAME: &str = "Collection";
    pub const INPUT: u64 = NodeFlags::ALL.bits();
    pub const OUTPUTS: [u64; 1] = [NodeFlags::COLLECTION.bits()];

    pub fn insert(&mut self, idx: usize, node: NodeId) {
        self.nodes.insert(idx, node);
        self.inputs.insert(idx, NodeFlags::ALL.bits());
    }

    pub fn remove(&mut self, idx: usize) {
        self.nodes.remove(idx);
        self.inputs.remove(idx);
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

    pub fn show_input(&self, pin: &InPin, ui: &mut Ui, snarl: &Snarl<Node>) -> PinInfo {
        let name = pin
            .remotes
            .first()
            .map(|out_pin| snarl[out_pin.node].name())
            .unwrap_or_default();

        empty_input_view(ui, format!("{} {name}", pin.id.input + 1))
    }

    pub fn connect_input(&mut self, from: &OutPin, to: &InPin) {
        self.insert(to.id.input, from.id.node);
    }

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        let input = input_pin.id.input;

        self.remove(input);
    }
}
