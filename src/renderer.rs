use std::sync::Arc;
use winit::dpi::PhysicalSize;
use winit::window::Window;
use glam::Vec3;
use image::GenericImageView;
use wgpu::util::DeviceExt;

use crate::camera::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexPositionColor {
    position: Vec3,
    color: Vec3
}

const VERTICES: &[VertexPositionColor] = &[
    VertexPositionColor { position: Vec3::new(-0.0868241, 0.49240386, 0.0), color: Vec3::new(0.5, 0.0, 0.5) }, // A
    VertexPositionColor { position: Vec3::new(-0.49513406, 0.06958647, 0.0), color: Vec3::new(0.5, 0.0, 0.5) }, // B
    VertexPositionColor { position: Vec3::new(-0.21918549, -0.44939706, 0.0), color: Vec3::new(0.5, 0.0, 0.5) }, // C
    VertexPositionColor { position: Vec3::new(0.35966998, -0.3473291, 0.0), color: Vec3::new(0.5, 0.0, 0.5) }, // D
    VertexPositionColor { position: Vec3::new(0.44147372, 0.2347359, 0.0), color: Vec3::new(0.5, 0.0, 0.5) }, // E
];

const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];


pub struct Renderer {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub should_reconfigure_surface: bool,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub shader_module: wgpu::ShaderModule,
    pub pipeline_layout: wgpu::PipelineLayout,
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,

    pub camera: Camera,
    pub camera_uniform_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub fn initialize(window: &Arc<Window>) -> Self {
        let window_size = window.inner_size();
        
        let diffuse_bytes = include_bytes!("../happy-tree.png");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();
        let image_dimensions = diffuse_image.dimensions();
        
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::debugging(),
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = smol::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })).unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities.formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        let (device, queue) = smol::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::Performance,
        }, None)).unwrap();

        surface.configure(&device, &surface_config);

        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into())
        });
        
        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: image_dimensions.0,
                height: image_dimensions.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<VertexPositionColor>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "fs_main",
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL
                })],
            }),
            multiview: None,
            cache: None,
        });
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        let camera = Camera {
            eye: Vec3::new(0.0, 1.0, 3.0),
            center: Vec3::new(0.0, 0.0, 0.0),
            up: Vec3::Y,
            aspect_ratio: window_size.width as f32 / window_size.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        
        let camera_uniform = CameraUniform {
            view_proj: camera.build_view_projection_matrix()
        };
        
        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding()
            }],
        });
        
        Renderer {
            instance,
            surface,
            surface_config,
            should_reconfigure_surface: false,
            adapter,
            device,
            queue,
            shader_module,
            pipeline_layout,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            camera,
            camera_uniform_buffer,
            camera_bind_group,
        }
    }
    
    pub fn resize_surface(&mut self, size: PhysicalSize<u32>) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.should_reconfigure_surface = true;
    }
    
    pub fn draw(&self) {
        let renderer = &self;
        
        if renderer.should_reconfigure_surface {
            renderer.surface.configure(&renderer.device, &renderer.surface_config);
        }

        match renderer.surface.get_current_texture() {
            Ok(frame) => {
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                let mut commands = renderer.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: None,
                });
                {
                    let mut render_pass = commands.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.1,
                                    g: 0.2,
                                    b: 0.3,
                                    a: 1.0
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    render_pass.set_pipeline(&renderer.render_pipeline);
                    render_pass.set_bind_group(0, &renderer.camera_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, renderer.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(
                        renderer.index_buffer.slice(..),
                        wgpu::IndexFormat::Uint16
                    );
                    render_pass.draw_indexed(0..(INDICES.len() as u32), 0, 0..1);
                }
                renderer.queue.submit(std::iter::once(commands.finish()));
                frame.present();
            }
            Err(_surface_error) => {
                return;
            }
        }
    }
}