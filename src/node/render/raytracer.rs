use eframe::egui_wgpu::{Callback, CallbackResources, CallbackTrait, RenderState, ScreenDescriptor};
use eframe::wgpu;
use egui::{PaintCallbackInfo, Ui};
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use serde::{Deserialize, Serialize};

use crate::node::camera::{CameraNode, camera_node_by_id};
use crate::node::message::{MessageHandling, SelfNodeMut};
use crate::node::scene::{SceneNode, SceneNodeResponse};
use crate::node::viewer::{empty_input_view, number_input_remote_value, number_input_view};
use crate::node::{Node, NodeFlags, collect_for_node};
use crate::raytracer::scene::Scene;
use crate::raytracer::{Camera, Raytracer, RenderParams, SamplingParams};
use crate::types::NodePin;

#[derive(Clone, Serialize, Deserialize)]
pub struct RaytracerRenderNode {
    max_samples_per_pixel: NodePin<u32>,
    num_samples_per_pixel: NodePin<u32>,
    num_bounces: NodePin<u32>,
    camera: NodePin<Option<NodeId>>,
    scene: Option<NodeId>,

    max_viewport_resolution: u32,
    #[serde(skip)]
    disconnect_scene: bool,
}

impl RaytracerRenderNode {
    pub fn new(max_viewport_resolution: u32) -> Self {
        let sampling = SamplingParams::default();
        Self {
            max_samples_per_pixel: NodePin::new(sampling.max_samples_per_pixel),
            num_samples_per_pixel: NodePin::new(sampling.num_samples_per_pixel),
            num_bounces: NodePin::new(sampling.num_bounces),
            camera: Default::default(),
            scene: Default::default(),

            max_viewport_resolution,
            disconnect_scene: false,
        }
    }

    pub fn camera_id(&self) -> Option<NodeId> {
        self.camera.get()
    }

    pub fn camera_node<'a>(&self, snarl: &'a Snarl<Node>) -> Option<&'a CameraNode> {
        self.camera
            .get()
            .and_then(|camera_id| camera_node_by_id(camera_id, snarl))
    }

    fn sampling_params(&self) -> SamplingParams {
        SamplingParams {
            max_samples_per_pixel: self.max_samples_per_pixel.get(),
            num_samples_per_pixel: self.num_samples_per_pixel.get(),
            num_bounces: self.num_bounces.get(),
        }
    }
}

impl RaytracerRenderNode {
    pub const NAME: &str = "Raytracer Render";
    pub const INPUTS: [u64; 5] = [
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::CAMERA.bits(),
        NodeFlags::SCENE.bits(),
    ];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::RENDER_RAYTRACER.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn register(&self, render_state: &RenderState) {
        RaytracerRenderResources::register(render_state, self, (0, 0));
    }

    pub fn unregister(&self, render_state: &RenderState) {
        RaytracerRenderResources::unregister(render_state);
    }

    pub fn draw(self_node: SelfNodeMut, viewport: egui::Rect, painter: &egui::Painter) {
        let node = self_node.as_render_node_ref().as_raytracer_render_ref();
        let render_params = node.camera_node(self_node.snarl).map(|camera_node| RenderParams {
            camera: Camera::from_node(camera_node),
            sky: Default::default(),
            sampling: node.sampling_params(),
        });

        let scene = if let Some(scene_node_id) = node.scene {
            if let SceneNodeResponse::Recalculated =
                SceneNode::handle_recalculate(SelfNodeMut::new(scene_node_id, self_node.snarl))
            {
                Some(self_node.snarl[scene_node_id].as_scene_node_ref().as_scene().clone())
            } else {
                None
            }
        } else if node.disconnect_scene {
            self_node.snarl[self_node.id]
                .as_render_node_mut()
                .as_raytracer_render_mut()
                .disconnect_scene = false;
            Some(Scene::stub())
        } else {
            None
        };

        if let Some(render_params) = render_params {
            let callback = Callback::new_paint_callback(viewport, Drawer { render_params, scene });
            painter.add(callback);
        }
    }
}

impl MessageHandling for RaytracerRenderNode {
    fn handle_input_show(mut self_node: SelfNodeMut, pin: &InPin, ui: &mut Ui) -> Option<PinInfo> {
        Some(match pin.id.input {
            0 => {
                const LABEL: &str = "Total samples per pixel";

                let remote_value =
                    number_input_remote_value(pin, self_node.snarl, LABEL).map(|(name, value)| (name, value as u32));
                let node = self_node.as_render_node_mut().as_raytracer_render_mut();
                number_input_view(ui, LABEL, &mut node.max_samples_per_pixel, remote_value)
            },
            1 => {
                const LABEL: &str = "Samples per pixel per frame";

                let remote_value =
                    number_input_remote_value(pin, self_node.snarl, LABEL).map(|(name, value)| (name, value as u32));
                let node = self_node.as_render_node_mut().as_raytracer_render_mut();
                number_input_view(ui, LABEL, &mut node.num_samples_per_pixel, remote_value)
            },
            2 => {
                const LABEL: &str = "Bounces per ray";

                let remote_value =
                    number_input_remote_value(pin, self_node.snarl, LABEL).map(|(name, value)| (name, value as u32));
                let node = self_node.as_render_node_mut().as_raytracer_render_mut();
                number_input_view(ui, LABEL, &mut node.num_bounces, remote_value)
            },
            3 => {
                const LABEL: &str = "Camera";

                let remote_value = match &*pin.remotes {
                    [] => None,
                    [remote] => Some(match &self_node.snarl[remote.node] {
                        Node::Camera(_) => Some(remote.node),
                        node => unreachable!("{LABEL} input not suppor connection with `{}`", node.name()),
                    }),
                    _ => None,
                };

                if let Some(value) = remote_value {
                    let node = self_node.as_render_node_mut().as_raytracer_render_mut();
                    node.camera.set(value);
                }

                empty_input_view(ui, LABEL)
            },
            4 => {
                const LABEL: &str = "Scene";

                let remote_value = match &*pin.remotes {
                    [] => None,
                    [remote] => Some(match &mut self_node.snarl[remote.node] {
                        Node::Scene(_) => Some(remote.node),
                        node => unreachable!("{LABEL} input not suppor connection with `{}`", node.name()),
                    }),
                    _ => None,
                };

                if let Some(value) = remote_value {
                    let node = self_node.as_render_node_mut().as_raytracer_render_mut();

                    node.scene = value;
                    if value != node.scene {
                        if let Some(scene_id) = node.scene {
                            self_node.node_by_id_mut(scene_id).as_scene_node_mut().register_render();
                        }
                    }
                }

                empty_input_view(ui, LABEL)
            },
            _ => unreachable!(),
        })
    }

    fn handle_input_disconnect(mut self_node: SelfNodeMut, _from: &OutPin, to: &InPin) {
        let node = self_node.as_render_node_mut().as_raytracer_render_mut();
        match to.id.input {
            0 => node.max_samples_per_pixel.reset(),
            1 => node.num_samples_per_pixel.reset(),
            2 => node.num_bounces.reset(),
            3 => node.camera.reset(),
            4 => {
                node.scene = None;
                node.disconnect_scene = true
            },
            _ => unreachable!(),
        }
    }

    fn handle_input_collect_ids(
        self_node: SelfNodeMut,
        predicate: &dyn Fn(&Node) -> bool,
        destination: &mut wgpu::naga::FastIndexSet<NodeId>,
    ) {
        let camera_node_id = self_node.as_render_node_ref().as_raytracer_render_ref().camera.get();
        let scene_node_id = self_node.as_render_node_ref().as_raytracer_render_ref().scene;

        collect_for_node(camera_node_id, predicate, destination, self_node.snarl);
        collect_for_node(scene_node_id, predicate, destination, self_node.snarl);
    }
}

struct Drawer {
    render_params: RenderParams,
    scene: Option<Scene>,
}

impl CallbackTrait for Drawer {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_descriptor: &ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(resources) = callback_resources.get_mut::<RaytracerRenderResources>() {
            let viewport_size = (screen_descriptor.size_in_pixels[0], screen_descriptor.size_in_pixels[1]);
            resources.prepare(device, queue, &self.render_params, self.scene.as_ref(), viewport_size);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        if let Some(resources) = callback_resources.get::<RaytracerRenderResources>() {
            resources.paint(render_pass);
        }
    }
}

pub struct RaytracerRenderResources {
    renderer: Raytracer,
}

impl RaytracerRenderResources {
    pub fn new(
        render_state: &RenderState,
        render_params: &RenderParams,
        viewport_size: (u32, u32),
        max_viewport_resolution: u32,
    ) -> Self {
        let device = &render_state.device;
        let target_format = render_state.target_format;
        let scene = Scene::stub();

        Self {
            renderer: Raytracer::new(
                device,
                target_format,
                &scene,
                render_params,
                viewport_size,
                max_viewport_resolution,
            )
            .expect("Raytracer creation failed"),
        }
    }

    pub fn register(render_state: &RenderState, node: &RaytracerRenderNode, viewport_size: (u32, u32)) {
        let render_params = RenderParams {
            camera: Default::default(),
            sky: Default::default(),
            sampling: node.sampling_params(),
        };

        render_state.renderer.write().callback_resources.insert(Self::new(
            render_state,
            &render_params,
            viewport_size,
            node.max_viewport_resolution,
        ));
    }

    pub fn unregister(render_state: &RenderState) {
        render_state.renderer.write().callback_resources.remove::<Self>();
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_params: &RenderParams,
        scene: Option<&Scene>,
        viewport_size: (u32, u32),
    ) {
        self.renderer
            .prepare_frame(device, queue, render_params, scene, viewport_size);
    }

    pub fn paint(&self, rpass: &mut wgpu::RenderPass<'static>) {
        self.renderer.render_frame(rpass);
    }
}
