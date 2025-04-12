use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin};
use serde::{Deserialize, Serialize};

use super::message::{EventMessage, EventResponse, MessageHandling, SelfNodeMut};
use super::subscribtion::{Event, Subscription};
use super::viewer::empty_input_view;
use super::{Node, NodeFlags, collect_for_node};

#[derive(Clone, Serialize, Deserialize)]
pub struct CollectionNode {
    nodes: Vec<NodeId>,
    inputs: Vec<u64>,

    #[serde(skip)]
    subscription: Subscription,
}

impl Default for CollectionNode {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            inputs: vec![NodeFlags::ALL.bits()],
            subscription: Subscription::default(),
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

    pub fn to_node_ids(&self) -> Vec<NodeId> {
        self.nodes.clone()
    }

    pub fn inputs(&self) -> &[u64] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }
}

impl MessageHandling for CollectionNode {
    fn handle_input_show(self_node: SelfNodeMut, pin: &InPin, ui: &mut Ui) -> Option<PinInfo> {
        let name = pin
            .remotes
            .first()
            .map(|out_pin| self_node.snarl[out_pin.node].name())
            .unwrap_or_default();

        Some(empty_input_view(ui, format!("{} {name}", pin.id.input + 1)))
    }

    fn handle_input_connect(mut self_node: SelfNodeMut, from: &OutPin, to: &InPin) {
        let node = self_node.as_collection_node_mut();
        node.insert(to.id.input, from.id.node);

        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_input_disconnect(mut self_node: SelfNodeMut, _from: &OutPin, to: &InPin) {
        let node = self_node.as_collection_node_mut();
        node.remove(to.id.input);

        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_input_collect_ids(
        self_node: SelfNodeMut,
        predicate: &dyn Fn(&Node) -> bool,
        destination: &mut eframe::wgpu::naga::FastIndexSet<NodeId>,
    ) {
        let nodes = self_node.as_collection_node_ref().to_node_ids();
        for node_id in nodes {
            collect_for_node(Some(node_id), predicate, destination, self_node.snarl)
        }
    }

    fn handle_event(mut self_node: SelfNodeMut, event_msg: EventMessage) -> Option<EventResponse> {
        let node = self_node.as_collection_node_mut();
        node.subscription.handle_event(event_msg)
    }
}
