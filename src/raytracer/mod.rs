use eframe::wgpu;
use eframe::wgpu::util::DeviceExt;
use gpu_buffer::{StorageBuffer, UniformBuffer};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use self::texture::Texture;
use crate::node::camera::CameraNode;
use crate::types::{Angle, Matrix4f32, Vector3, Vector3f32, Vector4f32, from_vector3_to_vector3f32};

mod gpu_buffer;
mod texture;

use std::f32::consts::*;

pub struct Raytracer {
    vertex_uniform_bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    frame_data_buffer: UniformBuffer,
    image_bind_group: wgpu::BindGroup,
    camera_buffer: UniformBuffer,
    sampling_parameter_buffer: UniformBuffer,
    hw_sky_state_buffer: StorageBuffer,
    parameter_bind_group: wgpu::BindGroup,
    scene_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    latest_render_params: RenderParams,
    render_progress: RenderProgress,
    frame_number: u32,
}

impl Raytracer {
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        scene: &Scene,
        render_params: &RenderParams,
        viewport_size: (u32, u32),
        max_viewport_resolution: u32,
    ) -> Result<Self, RenderParamsValidationError> {
        match render_params.validate() {
            Ok(_) => {},
            Err(err) => return Err(err),
        }

        let uniforms = VertexUniforms {
            view_projection_matrix: unit_quad_projection_matrix(),
            model_matrix: Matrix4f32::identity(),
        };
        let vertex_uniform_buffer =
            UniformBuffer::new_from_bytes(device, bytemuck::bytes_of(&uniforms), 0, Some("uniforms"));
        let vertex_uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[vertex_uniform_buffer.layout(wgpu::ShaderStages::VERTEX)],
            label: Some("uniforms layout"),
        });
        let vertex_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &vertex_uniform_bind_group_layout,
            entries: &[vertex_uniform_buffer.binding()],
            label: Some("uniforms bind group"),
        });

        let frame_data_buffer = UniformBuffer::new(device, 16_u64, 0, Some("frame data buffer"));

        let image_buffer = {
            let buffer = vec![[0.0; 3]; max_viewport_resolution as usize];
            StorageBuffer::new_from_bytes(device, bytemuck::cast_slice(buffer.as_slice()), 1, Some("image buffer"))
        };

        let image_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                frame_data_buffer.layout(wgpu::ShaderStages::FRAGMENT),
                image_buffer.layout(wgpu::ShaderStages::FRAGMENT, false),
            ],
            label: Some("image layout"),
        });
        let image_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &image_bind_group_layout,
            entries: &[frame_data_buffer.binding(), image_buffer.binding()],
            label: Some("image bind group"),
        });

        let camera_buffer = {
            let camera = GpuCamera::new(&render_params.camera, viewport_size);

            UniformBuffer::new_from_bytes(device, bytemuck::bytes_of(&camera), 0, Some("camera buffer"))
        };

        let sampling_parameter_buffer = UniformBuffer::new(
            device,
            std::mem::size_of::<GpuSamplingParams>() as wgpu::BufferAddress,
            1,
            Some("sampling parameter buffer"),
        );

        let hw_sky_state_buffer = {
            let sky_state = render_params.sky.to_sky_state()?;

            StorageBuffer::new_from_bytes(device, bytemuck::bytes_of(&sky_state), 2, Some("sky state buffer"))
        };

        let parameter_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                camera_buffer.layout(wgpu::ShaderStages::FRAGMENT),
                sampling_parameter_buffer.layout(wgpu::ShaderStages::FRAGMENT),
                hw_sky_state_buffer.layout(wgpu::ShaderStages::FRAGMENT, true),
            ],
            label: Some("parameter layout"),
        });

        let parameter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &parameter_bind_group_layout,
            entries: &[
                camera_buffer.binding(),
                sampling_parameter_buffer.binding(),
                hw_sky_state_buffer.binding(),
            ],
            label: Some("parameter bind group"),
        });

        let (scene_bind_group_layout, scene_bind_group) = {
            let sphere_buffer = StorageBuffer::new_from_bytes(
                device,
                bytemuck::cast_slice(scene.spheres.as_slice()),
                0,
                Some("scene buffer"),
            );

            let mut global_texture_data: Vec<[f32; 3]> = Vec::new();
            let mut material_data: Vec<GpuMaterial> = Vec::with_capacity(scene.materials.len());

            for material in scene.materials.iter() {
                let gpu_material = match material {
                    Material::Lambertian { albedo } => GpuMaterial::lambertian(albedo, &mut global_texture_data),
                    Material::Metal { albedo, fuzz } => GpuMaterial::metal(albedo, *fuzz, &mut global_texture_data),
                    Material::Dielectric { refraction_index } => GpuMaterial::dielectric(*refraction_index),
                    Material::Checkerboard { odd, even } => {
                        GpuMaterial::checkerboard(odd, even, &mut global_texture_data)
                    },
                    Material::Emissive { emit } => GpuMaterial::emissive(emit, &mut global_texture_data),
                };

                material_data.push(gpu_material);
            }

            let material_buffer = StorageBuffer::new_from_bytes(
                device,
                bytemuck::cast_slice(material_data.as_slice()),
                1,
                Some("materials buffer"),
            );

            let texture_buffer = StorageBuffer::new_from_bytes(
                device,
                bytemuck::cast_slice(global_texture_data.as_slice()),
                2,
                Some("textures buffer"),
            );

            let light_indices: Vec<u32> = scene
                .spheres
                .iter()
                .enumerate()
                .filter(|(_, s)| matches!(scene.materials[s.material_idx as usize], Material::Emissive { .. }))
                .map(|(idx, _)| idx as u32)
                .collect();

            let light_buffer = StorageBuffer::new_from_bytes(
                device,
                bytemuck::cast_slice(light_indices.as_slice()),
                3,
                Some("lights buffer"),
            );

            let scene_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    sphere_buffer.layout(wgpu::ShaderStages::FRAGMENT, true),
                    material_buffer.layout(wgpu::ShaderStages::FRAGMENT, true),
                    texture_buffer.layout(wgpu::ShaderStages::FRAGMENT, true),
                    light_buffer.layout(wgpu::ShaderStages::FRAGMENT, true),
                ],
                label: Some("scene layout"),
            });
            let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &scene_bind_group_layout,
                entries: &[
                    sphere_buffer.binding(),
                    material_buffer.binding(),
                    texture_buffer.binding(),
                    light_buffer.binding(),
                ],
                label: Some("scene bind group"),
            });

            (scene_bind_group_layout, scene_bind_group)
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            source: wgpu::ShaderSource::Wgsl(include_str!("raytracer_shader.wgsl").into()),
            label: Some("raytracer_shader.wgsl"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[
                &vertex_uniform_bind_group_layout,
                &image_bind_group_layout,
                &parameter_bind_group_layout,
                &scene_bind_group_layout,
            ],
            push_constant_ranges: &[],
            label: Some("raytracer layout"),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vsMain"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fsMain"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                polygon_mode: wgpu::PolygonMode::Fill,
                cull_mode: Some(wgpu::Face::Back),
                // Requires Features::DEPTH_CLAMPING
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("raytracer pipeline"),
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
            label: Some("VertexInput buffer"),
        });

        let render_progress = RenderProgress::new();

        let frame_number = 1;

        Ok(Self {
            vertex_uniform_bind_group,
            frame_data_buffer,
            image_bind_group,
            camera_buffer,
            sampling_parameter_buffer,
            hw_sky_state_buffer,
            parameter_bind_group,
            scene_bind_group,
            vertex_buffer,
            pipeline,
            latest_render_params: *render_params,
            render_progress,
            frame_number,
        })
    }

    pub fn prepare_frame(&mut self, queue: &wgpu::Queue, render_params: &RenderParams, viewport_size: (u32, u32)) {
        self.set_render_params(queue, render_params, viewport_size)
            .expect("Render params should be valid");

        let gpu_sampling_params = self.render_progress.next_frame(&self.latest_render_params.sampling);

        queue.write_buffer(
            self.sampling_parameter_buffer.handle(),
            0,
            bytemuck::cast_slice(&[gpu_sampling_params]),
        );

        let frame_number = self.frame_number;
        let frame_data = [viewport_size.0, viewport_size.1, frame_number];
        queue.write_buffer(self.frame_data_buffer.handle(), 0, bytemuck::cast_slice(&frame_data));

        self.frame_number += 1;
    }

    pub fn render_frame(&self, render_pass: &mut wgpu::RenderPass) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.vertex_uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &self.image_bind_group, &[]);
        render_pass.set_bind_group(2, &self.parameter_bind_group, &[]);
        render_pass.set_bind_group(3, &self.scene_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        let num_vertices = VERTICES.len() as u32;
        render_pass.draw(0..num_vertices, 0..1);
    }

    pub fn set_render_params(
        &mut self,
        queue: &wgpu::Queue,
        render_params: &RenderParams,
        viewport_size: (u32, u32),
    ) -> Result<(), RenderParamsValidationError> {
        if *render_params == self.latest_render_params {
            return Ok(());
        }

        render_params.validate()?;
        if viewport_size.0 == 0 || viewport_size.1 == 0 {
            return Err(RenderParamsValidationError::ViewportSize(
                viewport_size.0,
                viewport_size.1,
            ));
        }

        {
            let sky_state = render_params.sky.to_sky_state()?;
            queue.write_buffer(self.hw_sky_state_buffer.handle(), 0, bytemuck::bytes_of(&sky_state));
        }

        {
            let camera = GpuCamera::new(&render_params.camera, viewport_size);
            queue.write_buffer(self.camera_buffer.handle(), 0, bytemuck::bytes_of(&camera));
        }

        self.latest_render_params = *render_params;

        self.render_progress.reset();

        Ok(())
    }

    pub fn progress(&self) -> f32 {
        self.render_progress.accumulated_samples() as f32
            / self.latest_render_params.sampling.max_samples_per_pixel as f32
    }
}

#[derive(Error, Debug)]
pub enum RenderParamsValidationError {
    #[error("max_samples_per_pixel ({0}) is not a multiple of num_samples_per_pixel ({1})")]
    MaxSampleCountNotMultiple(u32, u32),
    #[error("viewport_size elements cannot be zero: ({0}, {1})")]
    ViewportSize(u32, u32),
    #[error("vfov must be between 0..=90 degrees")]
    VfovOutOfRange(f64),
    #[error("aperture must be between 0..=1")]
    ApertureOutOfRange(f64),
    #[error("focus_distance must be greater than zero")]
    FocusDistanceOutOfRange(f64),
    #[error(transparent)]
    HwSkyModelValidationError(#[from] hw_skymodel::rgb::Error),
}

#[derive(Debug, Default)]
pub struct Scene {
    pub spheres: Vec<Sphere>,
    pub materials: Vec<Material>,
}

impl Scene {
    pub fn test() -> Self {
        let materials = vec![
            Material::Checkerboard {
                even: Texture::new_from_color(Vector3f32::new(0.5, 0.7, 0.8)),
                odd: Texture::new_from_color(Vector3f32::new(0.9, 0.9, 0.9)),
            },
            Material::Lambertian {
                albedo: Texture::new_from_image("assets/moon.jpeg").expect("Hardcoded path should be valid"),
            },
            Material::Metal {
                albedo: Texture::new_from_color(Vector3f32::new(1.0, 0.85, 0.57)),
                fuzz: 0.4,
            },
            Material::Dielectric { refraction_index: 1.5 },
            Material::Lambertian {
                albedo: Texture::new_from_image("assets/earthmap.jpeg").expect("Hardcoded path should be valid"),
            },
            Material::Emissive {
                emit: Texture::new_from_scaled_image("assets/sun.jpeg", 50.0).expect("Hardcoded path should be valid"),
            },
            Material::Lambertian {
                albedo: Texture::new_from_color(Vector3f32::new(0.3, 0.9, 0.9)),
            },
            Material::Emissive {
                emit: Texture::new_from_color(Vector3f32::new(50.0, 0.0, 0.0)),
            },
            Material::Emissive {
                emit: Texture::new_from_color(Vector3f32::new(0.0, 50.0, 0.0)),
            },
            Material::Emissive {
                emit: Texture::new_from_color(Vector3f32::new(0.0, 0.0, 50.0)),
            },
        ];

        let spheres = vec![
            Sphere::new(Vector3::new(0.0, -500.0, -1.0), 500.0, 0),
            // left row
            Sphere::new(Vector3::new(-5.0, 1.0, -4.0), 1.0, 7),
            Sphere::new(Vector3::new(0.0, 1.0, -4.0), 1.0, 8),
            Sphere::new(Vector3::new(5.0, 1.0, -4.0), 1.0, 9),
            // middle row
            Sphere::new(Vector3::new(-5.0, 1.0, 0.0), 1.0, 2),
            Sphere::new(Vector3::new(0.0, 1.0, 0.0), 1.0, 3),
            Sphere::new(Vector3::new(5.0, 1.0, 0.0), 1.0, 6),
            // right row
            Sphere::new(Vector3::new(-5.0, 0.8, 4.0), 0.8, 1),
            Sphere::new(Vector3::new(0.0, 1.2, 4.0), 1.2, 4),
            Sphere::new(Vector3::new(5.0, 2.0, 4.0), 2.0, 5),
        ];

        Self { spheres, materials }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::NoUninit)]
pub struct Sphere {
    // NOTE: naga memory alignment issue, see discussion at
    // https://github.com/gfx-rs/naga/issues/2000
    // It's safer to just use Vec4 instead of Vec3.
    center: Vector4f32, // 0 byte offset
    radius: f32,        // 16 byte offset
    material_idx: u32,  // 20 byte offset
    _padding: [u32; 2], // 24 byte offset, 8 bytes size
}

impl Sphere {
    pub fn new(center: Vector3, radius: f64, material_idx: u32) -> Self {
        Self {
            center: Vector4f32::new(center.x as _, center.y as _, center.z as _, 0.0),
            radius: radius as _,
            material_idx,
            _padding: [0; 2],
        }
    }
}

#[derive(Debug)]
pub enum Material {
    Lambertian { albedo: Texture },
    Metal { albedo: Texture, fuzz: f32 },
    Dielectric { refraction_index: f32 },
    Checkerboard { even: Texture, odd: Texture },
    Emissive { emit: Texture },
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RenderParams {
    pub camera: Camera,
    pub sky: SkyParams,
    pub sampling: SamplingParams,
}

impl RenderParams {
    fn validate(&self) -> Result<(), RenderParamsValidationError> {
        if self.sampling.max_samples_per_pixel % self.sampling.num_samples_per_pixel != 0 {
            return Err(RenderParamsValidationError::MaxSampleCountNotMultiple(
                self.sampling.max_samples_per_pixel,
                self.sampling.num_samples_per_pixel,
            ));
        }

        if !(Angle::degrees(0.0)..=Angle::degrees(90.0)).contains(&self.camera.vfov) {
            return Err(RenderParamsValidationError::VfovOutOfRange(
                self.camera.vfov.as_degrees(),
            ));
        }

        if !(0.0..=1.0).contains(&self.camera.aperture) {
            return Err(RenderParamsValidationError::ApertureOutOfRange(self.camera.aperture));
        }

        if self.camera.focus_distance < 0.0 {
            return Err(RenderParamsValidationError::FocusDistanceOutOfRange(
                self.camera.focus_distance,
            ));
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub eye_pos: Vector3,
    pub eye_dir: Vector3,
    pub up: Vector3,
    /// Angle must be between 0..=90 degrees.
    pub vfov: Angle,
    /// Aperture must be between 0..=1.
    pub aperture: f64,
    /// Focus distance must be a positive number.
    pub focus_distance: f64,
}

impl Camera {
    pub fn from_node(node: &CameraNode) -> Self {
        let orientation = node.orientation();

        Self {
            eye_pos: node.position.get(),
            eye_dir: orientation.forward,
            up: orientation.up,
            vfov: node.vfov.get(),
            aperture: node.aperture.get(),
            focus_distance: node.focus_distance.get(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SkyParams {
    // Azimuth must be between 0..=360 degrees
    pub azimuth: Angle,
    // Inclination must be between 0..=90 degrees
    pub zenith: Angle,
    // Turbidity must be between 1..=10
    pub turbidity: f32,
    // Albedo elements must be between 0..=1
    pub albedo: [f32; 3],
}

impl Default for SkyParams {
    fn default() -> Self {
        Self {
            azimuth: Angle::degrees(0.0),
            zenith: Angle::degrees(85.0),
            turbidity: 4.0,
            albedo: [1.0; 3],
        }
    }
}

impl SkyParams {
    fn to_sky_state(self: &SkyParams) -> Result<GpuSkyState, hw_skymodel::rgb::Error> {
        let azimuth = self.azimuth.as_radians() as f32;
        let zenith = self.zenith.as_radians() as f32;
        let sun_direction = [
            zenith.sin() * azimuth.cos(),
            zenith.cos(),
            zenith.sin() * azimuth.sin(),
            0.0,
        ];

        let state = hw_skymodel::rgb::SkyState::new(&hw_skymodel::rgb::SkyParams {
            elevation: FRAC_PI_2 - zenith,
            turbidity: self.turbidity,
            albedo: self.albedo,
        })?;

        let (params_data, radiance_data) = state.raw();

        Ok(GpuSkyState {
            params: params_data,
            radiances: radiance_data,
            _padding: [0, 2],
            sun_direction,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SamplingParams {
    pub max_samples_per_pixel: u32,
    pub num_samples_per_pixel: u32,
    pub num_bounces: u32,
}

impl Default for SamplingParams {
    fn default() -> Self {
        Self {
            max_samples_per_pixel: 256,
            num_samples_per_pixel: 1,
            num_bounces: 8,
        }
    }
}

struct RenderProgress {
    accumulated_samples_per_pixel: u32,
}

impl RenderProgress {
    pub fn new() -> Self {
        Self {
            accumulated_samples_per_pixel: 0,
        }
    }

    pub fn next_frame(&mut self, sampling_params: &SamplingParams) -> GpuSamplingParams {
        let current_accumulated_samples = self.accumulated_samples_per_pixel;
        let next_accumulated_samples = sampling_params.num_samples_per_pixel + current_accumulated_samples;

        // Initial state: no samples have been accumulated yet. This is the first frame
        // after a reset. The image buffer's previous samples should be cleared by
        // setting clear_accumulated_samples to 1.
        if current_accumulated_samples == 0 {
            self.accumulated_samples_per_pixel = next_accumulated_samples;
            GpuSamplingParams {
                num_samples_per_pixel: sampling_params.num_samples_per_pixel,
                num_bounces: sampling_params.num_bounces,
                accumulated_samples_per_pixel: next_accumulated_samples,
                clear_accumulated_samples: 1,
            }
        }
        // Progressive render: accumulating samples in the image buffer over multiple
        // frames.
        else if next_accumulated_samples <= sampling_params.max_samples_per_pixel {
            self.accumulated_samples_per_pixel = next_accumulated_samples;
            GpuSamplingParams {
                num_samples_per_pixel: sampling_params.num_samples_per_pixel,
                num_bounces: sampling_params.num_bounces,
                accumulated_samples_per_pixel: next_accumulated_samples,
                clear_accumulated_samples: 0,
            }
        }
        // Completed render: we have accumulated max_samples_per_pixel samples. Stop rendering
        // by setting num_samples_per_pixel to zero.
        else {
            GpuSamplingParams {
                num_samples_per_pixel: 0,
                num_bounces: sampling_params.num_bounces,
                accumulated_samples_per_pixel: current_accumulated_samples,
                clear_accumulated_samples: 0,
            }
        }
    }

    pub fn reset(&mut self) {
        self.accumulated_samples_per_pixel = 0;
    }

    pub fn accumulated_samples(&self) -> u32 {
        self.accumulated_samples_per_pixel
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuCamera {
    eye: Vector3f32,
    _padding1: f32,
    horizontal: Vector3f32,
    _padding2: f32,
    vertical: Vector3f32,
    _padding3: f32,
    u: Vector3f32,
    _padding4: f32,
    v: Vector3f32,
    lens_radius: f32,
    lower_left_corner: Vector3f32,
    _padding5: f32,
}

impl GpuCamera {
    pub fn new(camera: &Camera, viewport_size: (u32, u32)) -> Self {
        let lens_radius = 0.5 * camera.aperture;
        let aspect = viewport_size.0 as f64 / viewport_size.1 as f64;
        let theta = camera.vfov.as_radians();
        let half_height = camera.focus_distance * (0.5 * theta).tan();
        let half_width = aspect * half_height;

        let w = camera.eye_dir.normalize();
        let v = camera.up.normalize();
        let u = w.cross(&v);

        let lower_left_corner = camera.eye_pos + camera.focus_distance * w - half_width * u - half_height * v;
        let horizontal = 2.0 * half_width * u;
        let vertical = 2.0 * half_height * v;

        Self {
            eye: from_vector3_to_vector3f32(&camera.eye_pos),
            _padding1: 0.0,
            horizontal: from_vector3_to_vector3f32(&horizontal),
            _padding2: 0.0,
            vertical: from_vector3_to_vector3f32(&vertical),
            _padding3: 0.0,
            u: from_vector3_to_vector3f32(&u),
            _padding4: 0.0,
            v: from_vector3_to_vector3f32(&v),
            lens_radius: lens_radius as _,
            lower_left_corner: from_vector3_to_vector3f32(&lower_left_corner),
            _padding5: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuMaterial {
    id: u32,
    desc1: TextureDescriptor,
    desc2: TextureDescriptor,
    x: f32,
}

impl GpuMaterial {
    pub fn lambertian(albedo: &Texture, global_texture_data: &mut Vec<[f32; 3]>) -> Self {
        Self {
            id: 0,
            desc1: Self::append_to_global_texture_data(albedo, global_texture_data),
            desc2: TextureDescriptor::empty(),
            x: 0.0,
        }
    }

    pub fn metal(albedo: &Texture, fuzz: f32, global_texture_data: &mut Vec<[f32; 3]>) -> Self {
        Self {
            id: 1,
            desc1: Self::append_to_global_texture_data(albedo, global_texture_data),
            desc2: TextureDescriptor::empty(),
            x: fuzz,
        }
    }

    pub fn dielectric(refraction_index: f32) -> Self {
        Self {
            id: 2,
            desc1: TextureDescriptor::empty(),
            desc2: TextureDescriptor::empty(),
            x: refraction_index,
        }
    }

    pub fn checkerboard(even: &Texture, odd: &Texture, global_texture_data: &mut Vec<[f32; 3]>) -> Self {
        Self {
            id: 3,
            desc1: Self::append_to_global_texture_data(even, global_texture_data),
            desc2: Self::append_to_global_texture_data(odd, global_texture_data),
            x: 0.0,
        }
    }

    pub fn emissive(emit: &Texture, global_texture_data: &mut Vec<[f32; 3]>) -> Self {
        Self {
            id: 4,
            desc1: Self::append_to_global_texture_data(emit, global_texture_data),
            desc2: TextureDescriptor::empty(),
            x: 0.0,
        }
    }

    fn append_to_global_texture_data(texture: &Texture, global_texture_data: &mut Vec<[f32; 3]>) -> TextureDescriptor {
        let dimensions = texture.dimensions();
        let offset = global_texture_data.len() as u32;
        global_texture_data.extend_from_slice(texture.as_slice());
        TextureDescriptor {
            width: dimensions.0,
            height: dimensions.1,
            offset,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TextureDescriptor {
    width: u32,
    height: u32,
    offset: u32,
}

impl TextureDescriptor {
    pub fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            offset: 0xffffffff,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuSkyState {
    params: [f32; 27],       // 0 byte offset, 108 byte size
    radiances: [f32; 3],     // 108 byte offset, 12 byte size
    _padding: [u32; 2],      // 120 byte offset, 8 byte size
    sun_direction: [f32; 4], // 128 byte offset, 16 byte size
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuSamplingParams {
    num_samples_per_pixel: u32,
    num_bounces: u32,
    accumulated_samples_per_pixel: u32,
    clear_accumulated_samples: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexUniforms {
    view_projection_matrix: Matrix4f32,
    model_matrix: Matrix4f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // @location(0)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                // @location(1)
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 2]>() as u64,
                    shader_location: 1,
                },
            ],
        }
    }
}

fn unit_quad_projection_matrix() -> Matrix4f32 {
    let sw = 0.5;
    let sh = 0.5;

    // Our ortho camera is just centered at (0, 0)

    let left = -sw;
    let right = sw;
    let bottom = -sh;
    let top = sh;

    // DirectX, Metal, wgpu share the same left-handed coordinate system
    // for their normalized device coordinates:
    // https://github.com/gfx-rs/gfx/tree/master/src/backend/dx12
    ortho_lh_zo(left, right, bottom, top, -1.0, 1.0)
}

/// Creates a matrix for a left hand orthographic-view frustum with a depth range of 0 to 1
///
/// # Parameters
///
/// * `left` - Coordinate for left bound of matrix
/// * `right` - Coordinate for right bound of matrix
/// * `bottom` - Coordinate for bottom bound of matrix
/// * `top` - Coordinate for top bound of matrix
/// * `znear` - Distance from the viewer to the near clipping plane
/// * `zfar` - Distance from the viewer to the far clipping plane
fn ortho_lh_zo(left: f32, right: f32, bottom: f32, top: f32, znear: f32, zfar: f32) -> Matrix4f32 {
    let one = 1.0;
    let two = 2.0;
    let mut mat = Matrix4f32::identity();

    mat[(0, 0)] = two / (right - left);
    mat[(0, 3)] = -(right + left) / (right - left);
    mat[(1, 1)] = two / (top - bottom);
    mat[(1, 3)] = -(top + bottom) / (top - bottom);
    mat[(2, 2)] = one / (zfar - znear);
    mat[(2, 3)] = -znear / (zfar - znear);

    mat
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, 0.5],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5],
        tex_coords: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5],
        tex_coords: [0.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5],
        tex_coords: [1.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5],
        tex_coords: [1.0, 0.0],
    },
];
