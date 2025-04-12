use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin};
use serde::{Deserialize, Serialize};

use super::material::InputMaterial;
use super::message::{
    CommonNodeMessage, CommonNodeResponse, EventMessage, EventResponse, MessageHandling, SelfNodeMut,
};
use super::subscribtion::Subscription;
use super::{NodeFlags, collect_for_node};
use crate::node::Node;
use crate::node::subscribtion::Event;
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

    pub fn handle_msg(self_node: SelfNodeMut, msg: CommonNodeMessage) -> Option<CommonNodeResponse> {
        match self_node.as_primitive_node_ref() {
            Self::Sphere(_) => SphereNode::handle_msg(self_node, msg),
        }
    }

    pub fn as_sphere_ref(&self) -> &SphereNode {
        match self {
            Self::Sphere(sphere) => sphere,
        }
    }

    pub fn as_sphere_mut(&mut self) -> &mut SphereNode {
        match self {
            Self::Sphere(sphere) => sphere,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SphereNode {
    pub center: NodePin<Vector3>,
    pub radius: NodePin<f64>,
    pub material: NodePin<InputMaterial>,

    #[serde(skip)]
    subscription: Subscription,
}

impl Default for SphereNode {
    fn default() -> Self {
        Self {
            center: Default::default(),
            radius: NodePin::new(1.0),
            material: Default::default(),
            subscription: Subscription::default(),
        }
    }
}

impl SphereNode {
    pub const NAME: &str = "Sphere Primitive";
    pub const INPUTS: [u64; 3] = [
        NodeFlags::TYPICAL_VECTOR_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::MATERIALS.bits(),
    ];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::PRIMITIVE_SPHERE.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }
}

impl MessageHandling for SphereNode {
    fn handle_input_show(mut self_node: SelfNodeMut, pin: &InPin, ui: &mut Ui) -> Option<PinInfo> {
        Some(match pin.id.input {
            0 => {
                const LABEL: &str = "Center";

                let remote_value = vector_input_remote_value(pin, self_node.snarl, LABEL);
                let node = self_node.as_primitive_node_mut().as_sphere_mut();

                let old_value = node.center.get();
                let info = vector_input_view(ui, LABEL, &mut node.center, remote_value);

                if old_value != node.center.get() {
                    if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
                        caller(self_node)
                    }
                }
                info
            },
            1 => {
                const LABEL: &str = "Radius";

                let remote_value = number_input_remote_value(pin, self_node.snarl, LABEL);
                let node = self_node.as_primitive_node_mut().as_sphere_mut();

                let old_value = node.radius.get();
                let info = number_input_view(ui, LABEL, &mut node.radius, remote_value);

                if old_value != node.radius.get() {
                    if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
                        caller(self_node)
                    }
                }
                info
            },
            2 => {
                const LABEL: &str = "Material";

                let remote_value = material_input_remote_value(pin, self_node.snarl, LABEL);
                let node = self_node.as_primitive_node_mut().as_sphere_mut();

                let old_value = node.material.as_ref().clone();
                let info = material_input_view(ui, LABEL, &mut node.material, remote_value);

                if old_value != *node.material.as_ref() {
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
        let node = self_node.as_primitive_node_mut().as_sphere_mut();
        if let Some(caller) = node.subscription.event_caller(Event::OnChange) {
            caller(self_node)
        }
    }

    fn handle_input_disconnect(mut self_node: SelfNodeMut, _from: &OutPin, to: &InPin) {
        let node = self_node.as_primitive_node_mut().as_sphere_mut();
        match to.id.input {
            0 => node.center.reset(),
            1 => node.radius.reset(),
            2 => node.material.reset(),
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
        let node = self_node.as_primitive_node_ref().as_sphere_ref();
        if let InputMaterial::External(node_id) = node.material.as_ref() {
            collect_for_node(Some(*node_id), predicate, destination, self_node.snarl);
        }
    }

    fn handle_event(mut self_node: SelfNodeMut, event_msg: EventMessage) -> Option<EventResponse> {
        let node = self_node.as_primitive_node_mut().as_sphere_mut();
        node.subscription.handle_event(event_msg)
    }
}
