use eframe::egui_wgpu::{Callback, CallbackResources, CallbackTrait, RenderState, ScreenDescriptor};
use eframe::wgpu;
use egui::{PaintCallbackInfo, Ui};
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, NodeId, OutPin, Snarl};
use serde::{Deserialize, Serialize};

use crate::node::camera::{CameraNode, camera_node_by_id};
use crate::node::viewer::{empty_input_view, number_input_remote_value, number_input_view};
use crate::node::{Node, NodeFlags};
use crate::raytracer::{Camera, Raytracer, RenderParams, SamplingParams, Scene};
use crate::types::NodePin;

#[derive(Clone, Serialize, Deserialize)]
pub struct RaytracerRenderNode {
    max_samples_per_pixel: NodePin<u32>,
    num_samples_per_pixel: NodePin<u32>,
    num_bounces: NodePin<u32>,
    camera: NodePin<Option<NodeId>>,

    max_viewport_resolution: u32,
}

impl RaytracerRenderNode {
    pub fn new(max_viewport_resolution: u32) -> Self {
        let sampling = SamplingParams::default();
        Self {
            max_samples_per_pixel: NodePin::new(sampling.max_samples_per_pixel),
            num_samples_per_pixel: NodePin::new(sampling.num_samples_per_pixel),
            num_bounces: NodePin::new(sampling.num_bounces),
            camera: Default::default(),

            max_viewport_resolution,
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
    pub const INPUTS: [u64; 4] = [
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::TYPICAL_NUMBER_INPUT.bits(),
        NodeFlags::CAMERA.bits(),
    ];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::RENDER_RAYTRACER.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "Total samples per pixel";

                let remote_value =
                    number_input_remote_value(pin, snarl, LABEL).map(|(name, value)| (name, value as u32));
                let node = snarl[pin.id.node].as_render_node_mut().as_raytracer_render_mut();
                number_input_view(ui, LABEL, &mut node.max_samples_per_pixel, remote_value)
            },
            1 => {
                const LABEL: &str = "Samples per pixel per frame";

                let remote_value =
                    number_input_remote_value(pin, snarl, LABEL).map(|(name, value)| (name, value as u32));
                let node = snarl[pin.id.node].as_render_node_mut().as_raytracer_render_mut();
                number_input_view(ui, LABEL, &mut node.num_samples_per_pixel, remote_value)
            },
            2 => {
                const LABEL: &str = "Bounces per ray";

                let remote_value =
                    number_input_remote_value(pin, snarl, LABEL).map(|(name, value)| (name, value as u32));
                let node = snarl[pin.id.node].as_render_node_mut().as_raytracer_render_mut();
                number_input_view(ui, LABEL, &mut node.num_bounces, remote_value)
            },
            3 => {
                const LABEL: &str = "Camera";

                let remote_value = match &*pin.remotes {
                    [] => None,
                    [remote] => Some(match &snarl[remote.node] {
                        Node::Camera(_) => Some(remote.node),
                        node => unreachable!("{LABEL} input not suppor connection with `{}`", node.name()),
                    }),
                    _ => None,
                };

                if let Some(value) = remote_value {
                    let node = snarl[pin.id.node].as_render_node_mut().as_raytracer_render_mut();
                    node.camera.set(value);
                }

                empty_input_view(ui, LABEL)
            },
            _ => unreachable!(),
        }
    }

    pub fn connect_input(&mut self, _from: &OutPin, _to: &InPin) {}

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match input_pin.id.input {
            0 => self.max_samples_per_pixel.reset(),
            1 => self.num_samples_per_pixel.reset(),
            2 => self.num_bounces.reset(),
            3 => self.camera.reset(),
            _ => unreachable!(),
        }
    }

    pub fn register(&self, render_state: &RenderState) {
        RaytracerRenderResources::register(render_state, self, (0, 0));
    }

    pub fn unregister(&self, render_state: &RenderState) {
        RaytracerRenderResources::unregister(render_state);
    }

    pub fn draw(&self, viewport: egui::Rect, painter: &egui::Painter, snarl: &Snarl<Node>) {
        if let Some(camera_node) = self.camera_node(snarl) {
            let render_params = RenderParams {
                camera: Camera::from_node(camera_node),
                sky: Default::default(),
                sampling: self.sampling_params(),
            };
            let callback = Callback::new_paint_callback(viewport, Drawer { render_params });
            painter.add(callback);
        }
    }
}

struct Drawer {
    render_params: RenderParams,
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
            resources.prepare(device, queue, &self.render_params, viewport_size);
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
        let scene = Scene::test();

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
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_params: &RenderParams,
        viewport_size: (u32, u32),
    ) {
        self.renderer.prepare_frame(queue, render_params, viewport_size);
    }

    pub fn paint(&self, rpass: &mut wgpu::RenderPass<'static>) {
        self.renderer.render_frame(rpass);
    }
}
