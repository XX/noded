use eframe::egui_wgpu::RenderState;
use serde::{Deserialize, Serialize};

use self::raytracer::RaytracerRenderNode;
use self::triangle::TriangleRenderNode;

pub mod raytracer;
pub mod triangle;

#[derive(Clone, Serialize, Deserialize)]
pub enum RenderNode {
    Triangle(TriangleRenderNode),
    Raytracer(RaytracerRenderNode),
}

impl RenderNode {
    pub const NAME: &str = "Render";

    pub fn name(&self) -> &str {
        match self {
            Self::Triangle(_) => TriangleRenderNode::NAME,
            Self::Raytracer(_) => RaytracerRenderNode::NAME,
        }
    }

    pub fn inputs(&self) -> &[u64] {
        match self {
            Self::Triangle(render) => render.inputs(),
            Self::Raytracer(render) => render.inputs(),
        }
    }

    pub fn outputs(&self) -> &[u64] {
        match self {
            Self::Triangle(render) => render.outputs(),
            Self::Raytracer(render) => render.outputs(),
        }
    }

    pub fn disconnect_input(&mut self, input: usize) {
        match self {
            Self::Triangle(render) => render.disconnect_input(input),
            Self::Raytracer(render) => render.disconnect_input(input),
        }
    }

    pub fn as_triangle_render_mut(&mut self) -> &mut TriangleRenderNode {
        match self {
            Self::Triangle(render) => render,
            node => panic!("Node `{}` is not a `{}`", node.name(), TriangleRenderNode::NAME),
        }
    }

    pub fn as_raytracer_render_mut(&mut self) -> &mut RaytracerRenderNode {
        match self {
            Self::Raytracer(render) => render,
            node => panic!("Node `{}` is not a `{}`", node.name(), RaytracerRenderNode::NAME),
        }
    }

    pub fn register(&self, render_state: &RenderState) {
        match self {
            Self::Triangle(render) => render.register(render_state),
            Self::Raytracer(render) => render.register(render_state),
        }
    }

    pub fn unregister(&self, render_state: &RenderState) {
        match self {
            Self::Triangle(render) => render.unregister(render_state),
            Self::Raytracer(render) => render.unregister(render_state),
        }
    }
}
