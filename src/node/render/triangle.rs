use eframe::egui_wgpu::{Callback, CallbackResources, CallbackTrait, RenderState, ScreenDescriptor, wgpu};
use eframe::wgpu::util::DeviceExt;
use egui::{PaintCallbackInfo, Ui};
use egui_snarl::ui::PinInfo;
use egui_snarl::{InPin, OutPin, Snarl};
use serde::{Deserialize, Serialize};

use crate::node::viewer::{number_input_remote_value, number_input_view};
use crate::node::{Node, NodeFlags};
use crate::types::NodePin;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct TriangleRenderNode {
    pub angle: NodePin<f64>,
}

impl TriangleRenderNode {
    pub const NAME: &str = "Triangle Render";
    pub const INPUTS: [u64; 1] = [NodeFlags::TYPICAL_NUMBER_INPUT.bits()];
    pub const OUTPUTS: [u64; 1] = [NodeFlags::RENDER_TRIANGLE.bits()];

    pub fn inputs(&self) -> &[u64] {
        &Self::INPUTS
    }

    pub fn outputs(&self) -> &[u64] {
        &Self::OUTPUTS
    }

    pub fn show_input(pin: &InPin, ui: &mut Ui, snarl: &mut Snarl<Node>) -> PinInfo {
        match pin.id.input {
            0 => {
                const LABEL: &str = "Angle";

                let remote_value = number_input_remote_value(pin, snarl, LABEL);
                let node = snarl[pin.id.node].as_render_node_mut().as_triangle_render_mut();
                number_input_view(ui, LABEL, &mut node.angle, remote_value)
            },
            _ => unreachable!(),
        }
    }

    pub fn connect_input(&mut self, _from: &OutPin, _to: &InPin) {}

    pub fn disconnect_input(&mut self, input_pin: &InPin) {
        match input_pin.id.input {
            0 => self.angle.reset(),
            _ => unreachable!(),
        }
    }

    pub fn register(&self, render_state: &RenderState) {
        TriangleRenderResources::register(render_state);
    }

    pub fn unregister(&self, render_state: &RenderState) {
        TriangleRenderResources::unregister(render_state);
    }
}

impl TriangleRenderNode {
    pub fn recalc_angle(&mut self, drag: f64) {
        self.angle.set(self.angle.get() + drag * 0.01);
    }

    pub fn draw(&self, viewport: egui::Rect, painter: &egui::Painter) {
        let callback = Callback::new_paint_callback(viewport, Drawer {
            angle: self.angle.get(),
        });
        painter.add(callback);
    }
}

struct Drawer {
    angle: f64,
}

// The callback for WGPU is in two stages: prepare, and paint.
//
// The prepare callback is called every frame before paint and is given access to the wgpu
// Device and Queue, which can be used, for instance, to update buffers and uniforms before
// rendering.
//
// The paint callback is called after prepare and is given access to the render pass, which
// can be used to issue draw commands.
impl CallbackTrait for Drawer {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(resources) = callback_resources.get::<TriangleRenderResources>() {
            resources.prepare(device, queue, self.angle as _);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        if let Some(resources) = callback_resources.get::<TriangleRenderResources>() {
            resources.paint(render_pass);
        }
    }
}

pub struct TriangleRenderResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl TriangleRenderResources {
    pub fn new(render_state: &RenderState) -> Self {
        let device = &render_state.device;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("./triangle_shader.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(render_state.target_format.into())],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[0.0]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Self {
            pipeline,
            bind_group,
            uniform_buffer,
        }
    }

    pub fn register(render_state: &RenderState) {
        render_state
            .renderer
            .write()
            .callback_resources
            .insert(Self::new(render_state));
    }

    pub fn unregister(render_state: &RenderState) {
        render_state.renderer.write().callback_resources.remove::<Self>();
    }

    pub fn prepare(&self, _device: &wgpu::Device, queue: &wgpu::Queue, angle: f32) {
        // Update our uniform buffer with the angle from the UI
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[angle]));
    }

    pub fn paint(&self, rpass: &mut wgpu::RenderPass<'static>) {
        // Draw our triangle!
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.draw(0..3, 0..1);
    }
}
