use eframe::wgpu::naga::FastHashMap;
use egui_snarl::NodeId;

use super::message::{EventMessage, EventResponse, SelfNodeMut};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Event {
    OnChange,
}

pub type EventCallback = fn(SelfNodeMut, NodeId);
pub type Subscribers = FastHashMap<NodeId, EventCallback>;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Subscription {
    subscribes: FastHashMap<Event, Subscribers>,
}

impl Subscription {
    pub fn handle_event(&mut self, event_msg: EventMessage) -> Option<EventResponse> {
        match event_msg {
            EventMessage::HasSubscription { node_id, event } => {
                let response = self.has_subscription(node_id, event);
                Some(EventResponse::HasSubscription(response))
            },
            EventMessage::Subscribe {
                node_id,
                event,
                callback,
            } => {
                self.subscribe(node_id, event, callback);
                None
            },
            EventMessage::Unsubscribe { node_id, event } => {
                self.unsubscribe(node_id, event);
                None
            },
        }
    }

    pub fn has_subscription(&self, node_id: NodeId, event: Event) -> bool {
        self.subscribes
            .get(&event)
            .map_or(false, |subscribers| subscribers.contains_key(&node_id))
    }

    pub fn subscribe(&mut self, node_id: NodeId, event: Event, callback: EventCallback) {
        self.subscribes
            .entry(event)
            .or_insert_with(Subscribers::default)
            .insert(node_id, callback);
    }

    pub fn unsubscribe(&mut self, node_id: NodeId, event: Event) {
        if let Some(subscribers) = self.subscribes.get_mut(&event) {
            subscribers.remove(&node_id);
        }
    }

    #[must_use]
    pub fn event_caller(&self, event: Event) -> Option<impl FnOnce(SelfNodeMut) + 'static> {
        if let Some(subscribers) = self.subscribes.get(&event) {
            let subscribers = subscribers.clone();

            Some(move |self_node: SelfNodeMut| {
                for (subscriber_id, callback) in subscribers {
                    callback(SelfNodeMut::new(self_node.id, self_node.snarl), subscriber_id);
                }
            })
        } else {
            None
        }
    }
}
