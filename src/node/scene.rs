use std::collections::HashMap;
use std::mem;

use bitflags::bitflags;
use eframe::wgpu::naga::FastIndexSet;
use egui::Ui;
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin};
use serde::{Deserialize, Serialize};

use super::collect_for_node;
use super::material::InputMaterial;
use super::message::{CommonNodeMessage, InputMessage, MessageHandling, SelfNodeMut};
use super::primitive::PrimitiveNode;
use crate::node::message::{CommonNodeResponse, EventMessage, EventResponse};
use crate::node::subscribtion::Event;
use crate::node::viewer::empty_input_view;
use crate::node::{Node, NodeFlags};
use crate::raytracer::scene::{Material, Scene, Sphere, TextureData};
use crate::types::NodePin;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,Serialize, Deserialize)]
    pub struct SceneDirtyFlags: u32 {
        const NONE = 0;

        const TEXTURE_VALUE = 1;
        const TEXTURE_LAYOUT = Self::TEXTURE_VALUE.bits() << 1;

        const MATERIAL_VALUE = Self::TEXTURE_LAYOUT.bits() << 1;
        const MATERIAL_LAYOUT = Self::MATERIAL_VALUE.bits() << 1;

        const PRIMITIVE_VALUE = Self::MATERIAL_LAYOUT.bits() << 1;
        const PRIMITIVE_LAYOUT = Self::PRIMITIVE_VALUE.bits() << 1;

        const ALL = u32::MAX;
        const INIT = Self::ALL.bits() - 1;
    }
}

impl Default for SceneDirtyFlags {
    fn default() -> Self {
        SceneDirtyFlags::INIT
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SceneNodeMessage {
    Recalculate,
}

#[derive(Clone, Copy, Debug)]
pub enum SceneNodeResponse {
    Recalculated,
    Nothing,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct SceneNode {
    pub data: NodePin<Option<NodeId>>,

    inner_scene: Scene,

    #[serde(skip)]
    tracked_nodes: FastIndexSet<NodeId>,

    #[serde(skip)]
    dirty: SceneDirtyFlags,
}

impl SceneNode {
    pub const NAME: &str = "Scene";
    pub const INPUTS: [u64; 1] = [NodeFlags::PRIMITIVES.bits() | NodeFlags::COLLECTION.bits()];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::SCENE.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }
}

impl MessageHandling for SceneNode {
    fn handle_input_show(mut self_node: SelfNodeMut, pin: &InPin, ui: &mut Ui) -> Option<PinInfo> {
        Some(match pin.id.input {
            0 => {
                const LABEL: &str = "Scene Data";

                let remote_value = match &*pin.remotes {
                    [] => None,
                    [remote] => Some(match &self_node.snarl[remote.node] {
                        Node::Primitive(_) | Node::Collection(_) => remote.node,
                        node => unreachable!("{LABEL} input not suppor connection with `{}`", node.name()),
                    }),
                    _ => None,
                };

                if let Some(value) = remote_value {
                    let node = self_node.as_scene_node_mut();
                    node.data.set(Some(value));
                }

                empty_input_view(ui, LABEL)
            },
            _ => unreachable!(),
        })
    }

    fn handle_input_connect(mut self_node: SelfNodeMut, from: &OutPin, _to: &InPin) {
        let node = self_node.as_scene_node_mut();
        node.data.set(Some(from.id.node));
        node.dirty = SceneDirtyFlags::ALL;
    }

    fn handle_input_disconnect(mut self_node: SelfNodeMut, _from: &OutPin, to: &InPin) {
        let node = self_node.as_scene_node_mut();
        match to.id.input {
            0 => {
                node.data.reset();
                node.dirty = SceneDirtyFlags::ALL;
            },
            _ => unreachable!(),
        }
    }

    fn handle_input_collect_ids(
        mut self_node: SelfNodeMut,
        predicate: &dyn Fn(&Node) -> bool,
        destination: &mut FastIndexSet<NodeId>,
    ) {
        let node = self_node.as_scene_node_mut();
        collect_for_node(node.data.get(), predicate, destination, self_node.snarl);
    }
}

impl SceneNode {
    pub fn as_scene(&self) -> &Scene {
        &self.inner_scene
    }

    pub fn register_render(&mut self) {
        self.dirty = SceneDirtyFlags::ALL;
    }

    pub fn handle_self_msg(self_node: SelfNodeMut, msg: SceneNodeMessage) -> SceneNodeResponse {
        match msg {
            SceneNodeMessage::Recalculate => Self::handle_recalculate(self_node),
        }
    }

    pub fn handle_recalculate(mut self_node: SelfNodeMut) -> SceneNodeResponse {
        let old_data = {
            let node = self_node.as_scene_node_mut();
            if node.dirty != SceneDirtyFlags::NONE {
                Some((mem::take(&mut node.inner_scene), mem::take(&mut node.tracked_nodes)))
            } else {
                None
            }
        };

        if let Some((mut old_scene, old_nodes)) = old_data {
            let mut nodes = FastIndexSet::default();
            Self::handle_msg(
                SelfNodeMut::new(self_node.id, self_node.snarl),
                CommonNodeMessage::Input(InputMessage::CollectIds {
                    predicate: &|node| {
                        matches!(
                            node,
                            Node::Primitive(_) | Node::Material(_) | Node::Texture(_) | Node::Collection(_)
                        )
                    },
                    destination: &mut nodes,
                }),
            );

            for node_id in &nodes {
                let has_subscription_response = Node::handle_msg(
                    SelfNodeMut::new(*node_id, self_node.snarl),
                    EventMessage::HasSubscription {
                        node_id: self_node.id,
                        event: Event::OnChange,
                    },
                );

                if let Some(CommonNodeResponse::Event(EventResponse::HasSubscription(false))) =
                    has_subscription_response
                {
                    Node::handle_msg(SelfNodeMut::new(*node_id, self_node.snarl), EventMessage::Subscribe {
                        node_id: self_node.id,
                        event: Event::OnChange,
                        callback: |self_node: SelfNodeMut, subscriber_id: NodeId| match self_node
                            .snarl
                            .get_node_mut(subscriber_id)
                        {
                            Some(Node::Scene(node)) => {
                                node.dirty = SceneDirtyFlags::ALL;
                            },
                            _ => {
                                Node::handle_msg(self_node, EventMessage::Unsubscribe {
                                    node_id: subscriber_id,
                                    event: Event::OnChange,
                                });
                            },
                        },
                    });
                }
            }

            for old_node_id in &old_nodes {
                if !nodes.contains(old_node_id) {
                    Node::handle_msg(
                        SelfNodeMut::new(*old_node_id, self_node.snarl),
                        EventMessage::Unsubscribe {
                            node_id: self_node.id,
                            event: Event::OnChange,
                        },
                    );
                }
            }

            let mut textures: Vec<TextureData> = Vec::new();
            let mut texture_indices = HashMap::new();

            let mut materials = Vec::new();
            let mut material_indices = HashMap::new();

            let mut spheres = Vec::new();

            for node_id in nodes {
                match self_node.node_by_id_ref(node_id) {
                    Node::Texture(texture_node) => {
                        let eq_predicate = |data: &TextureData| {
                            data.key.as_deref() == Some(texture_node.path.as_str())
                                && data.scale == texture_node.scale.get() as f32
                        };

                        if let Some(texture_id) = textures.iter().position(eq_predicate) {
                            texture_indices.insert(node_id, texture_id);
                        } else if let Some(texture_id) = old_scene.textures.iter().position(eq_predicate) {
                            let data = old_scene.textures.remove(texture_id);
                            textures.push(data);
                            texture_indices.insert(node_id, textures.len() - 1);
                        } else {
                            let data =
                                TextureData::load_scaled(texture_node.path.clone(), texture_node.scale.get() as _);
                            textures.push(data);
                            texture_indices.insert(node_id, textures.len() - 1);
                        }
                    },
                    Node::Material(material_node) => {
                        let texture_id = material_node
                            .get_texture_node_id()
                            .and_then(|node_id| texture_indices.get(&node_id).copied());
                        let material = Material::from_node(material_node, texture_id, &mut textures);
                        materials.push(material);
                        material_indices.insert(node_id, materials.len() - 1);
                    },
                    Node::Primitive(PrimitiveNode::Sphere(sphere_node)) => {
                        let material_idx = match sphere_node.material.as_ref() {
                            InputMaterial::Internal(material_node) => {
                                let texture_id = material_node
                                    .get_texture_node_id()
                                    .and_then(|node_id| texture_indices.get(&node_id).copied());
                                let material = Material::from_node(material_node, texture_id, &mut textures);
                                materials.push(material);
                                materials.len() - 1
                            },
                            InputMaterial::External(node_id) => material_indices[node_id],
                        };

                        let sphere = Sphere::from_node(sphere_node, material_idx as u32);
                        spheres.push(sphere);
                    },
                    _ => (),
                }
            }

            let node = self_node.as_scene_node_mut();
            node.inner_scene = Scene {
                spheres,
                materials,
                textures,
            };

            // Самый первый рендер с флагом инициализации не проходит до конца,
            // поэтому нужен будет повторный. В дальнейшем эта ошибка не повторяется.
            if node.dirty == SceneDirtyFlags::INIT {
                node.dirty = SceneDirtyFlags::ALL;
            } else {
                node.dirty = SceneDirtyFlags::NONE;
            }

            SceneNodeResponse::Recalculated
        } else {
            SceneNodeResponse::Nothing
        }
    }
}
