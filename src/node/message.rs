use eframe::wgpu::naga::FastIndexSet;
use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};

use super::Node;
use super::camera::CameraNode;
use super::collection::CollectionNode;
use super::material::MaterialNode;
use super::primitive::PrimitiveNode;
use super::render::RenderNode;
use super::scene::SceneNode;
use super::subscribtion::{Event, EventCallback};
use super::texture::TextureNode;

pub enum InputMessage<'a> {
    Show {
        pin: &'a InPin,
        ui: &'a mut Ui,
    },
    Connect {
        from: &'a OutPin,
        to: &'a InPin,
    },
    Disconnect {
        from: &'a OutPin,
        to: &'a InPin,
    },
    CollectIds {
        predicate: &'a dyn Fn(&Node) -> bool,
        destination: &'a mut FastIndexSet<NodeId>,
    },
}

pub enum InputResponse {
    Info(PinInfo),
}

pub enum EventMessage {
    HasSubscription {
        node_id: NodeId,
        event: Event,
    },
    Subscribe {
        node_id: NodeId,
        event: Event,
        callback: EventCallback,
    },
    Unsubscribe {
        node_id: NodeId,
        event: Event,
    },
}

pub enum EventResponse {
    HasSubscription(bool),
}

pub enum CommonNodeMessage<'a> {
    Input(InputMessage<'a>),
    Event(EventMessage),
}

impl<'a> From<InputMessage<'a>> for CommonNodeMessage<'a> {
    fn from(msg: InputMessage<'a>) -> Self {
        Self::Input(msg)
    }
}

impl<'a> From<EventMessage> for CommonNodeMessage<'a> {
    fn from(msg: EventMessage) -> Self {
        Self::Event(msg)
    }
}

pub enum CommonNodeResponse {
    Input(InputResponse),
    Event(EventResponse),
}

pub struct SelfNodeMut<'a> {
    pub id: NodeId,
    pub snarl: &'a mut Snarl<Node>,
}

impl<'a> SelfNodeMut<'a> {
    pub fn new(id: NodeId, snarl: &'a mut Snarl<Node>) -> Self {
        Self { id, snarl }
    }
}

impl SelfNodeMut<'_> {
    pub fn node_by_id_ref(&self, id: NodeId) -> &Node {
        &self.snarl[id]
    }

    pub fn node_by_id_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.snarl[id]
    }

    pub fn node_ref(&self) -> &Node {
        self.node_by_id_ref(self.id)
    }

    pub fn node_mut(&mut self) -> &mut Node {
        self.node_by_id_mut(self.id)
    }

    pub fn as_material_node_ref(&self) -> &MaterialNode {
        self.node_ref().as_material_node_ref()
    }

    pub fn as_material_node_mut(&mut self) -> &mut MaterialNode {
        self.node_mut().as_material_node_mut()
    }

    pub fn as_primitive_node_ref(&self) -> &PrimitiveNode {
        self.node_ref().as_primitive_node_ref()
    }

    pub fn as_primitive_node_mut(&mut self) -> &mut PrimitiveNode {
        self.node_mut().as_primitive_node_mut()
    }

    pub fn as_collection_node_ref(&self) -> &CollectionNode {
        self.node_ref().as_collection_node_ref()
    }

    pub fn as_collection_node_mut(&mut self) -> &mut CollectionNode {
        self.node_mut().as_collection_node_mut()
    }

    pub fn as_scene_node_mut(&mut self) -> &mut SceneNode {
        self.node_mut().as_scene_node_mut()
    }

    pub fn as_camera_node_mut(&mut self) -> &mut CameraNode {
        self.node_mut().as_camera_node_mut()
    }

    pub fn as_render_node_ref(&self) -> &RenderNode {
        self.node_ref().as_render_node_ref()
    }

    pub fn as_render_node_mut(&mut self) -> &mut RenderNode {
        self.node_mut().as_render_node_mut()
    }

    pub fn as_texture_node_mut(&mut self) -> &mut TextureNode {
        self.node_mut().as_texture_node_mut()
    }
}

pub trait MessageHandling {
    fn handle_input(self_node: SelfNodeMut, input_msg: InputMessage) -> Option<InputResponse> {
        match input_msg {
            InputMessage::Show { pin, ui } => Self::handle_input_show(self_node, pin, ui).map(InputResponse::Info),
            InputMessage::Connect { from, to } => {
                Self::handle_input_connect(self_node, from, to);
                None
            },
            InputMessage::Disconnect { from, to } => {
                Self::handle_input_disconnect(self_node, from, to);
                None
            },
            InputMessage::CollectIds { predicate, destination } => {
                Self::handle_input_collect_ids(self_node, predicate, destination);
                None
            },
        }
    }

    #[allow(unused_variables)]
    fn handle_input_show(self_node: SelfNodeMut, pin: &InPin, ui: &mut Ui) -> Option<PinInfo> {
        None
    }

    #[allow(unused_variables)]
    fn handle_input_connect(self_node: SelfNodeMut, from: &OutPin, to: &InPin) {}

    #[allow(unused_variables)]
    fn handle_input_disconnect(self_node: SelfNodeMut, from: &OutPin, to: &InPin) {}

    #[allow(unused_variables)]
    fn handle_input_collect_ids(
        self_node: SelfNodeMut,
        predicate: &dyn Fn(&Node) -> bool,
        destination: &mut FastIndexSet<NodeId>,
    ) {
    }

    fn handle_event(self_node: SelfNodeMut, event_msg: EventMessage) -> Option<EventResponse> {
        match event_msg {
            EventMessage::HasSubscription { node_id, event } => {
                let response = Self::handle_event_has_subscription(self_node, node_id, event);
                Some(EventResponse::HasSubscription(response))
            },
            EventMessage::Subscribe {
                node_id,
                event,
                callback,
            } => {
                Self::handle_event_subscribe(self_node, node_id, event, callback);
                None
            },
            EventMessage::Unsubscribe { node_id, event } => {
                Self::handle_event_unsubscribe(self_node, node_id, event);
                None
            },
        }
    }

    #[allow(unused_variables)]
    fn handle_event_has_subscription(self_node: SelfNodeMut, node_id: NodeId, event: Event) -> bool {
        false
    }

    #[allow(unused_variables)]
    fn handle_event_subscribe(self_node: SelfNodeMut, node_id: NodeId, event: Event, callback: EventCallback) {}

    #[allow(unused_variables)]
    fn handle_event_unsubscribe(self_node: SelfNodeMut, node_id: NodeId, event: Event) {}

    fn handle_msg(self_node: SelfNodeMut, msg: CommonNodeMessage) -> Option<CommonNodeResponse> {
        match msg {
            CommonNodeMessage::Input(input_msg) => {
                Self::handle_input(self_node, input_msg).map(CommonNodeResponse::Input)
            },
            CommonNodeMessage::Event(event_msg) => {
                Self::handle_event(self_node, event_msg).map(CommonNodeResponse::Event)
            },
        }
    }
}
