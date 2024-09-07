use std::sync::Arc;
use wgpu::{SurfaceError};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::event_loop::EventLoop;
use winit::window::WindowId;
use winit::window::Window;

struct Renderer {
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    should_resize_surface: bool,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    
    shader_module: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
}

enum App {
    Init,
    Main {
        window: Arc<Window>,
        renderer: Renderer
    },
}

impl winit::application::ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let App::Init = self else { return };
        
        let window = Arc::new(event_loop.create_window(Window::default_attributes()).unwrap());
        let window_size = window.inner_size();

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
        
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
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
        
        *self = App::Main {
            window,
            renderer: Renderer {
                instance,
                surface,
                surface_config,
                should_resize_surface: false,
                adapter,
                device,
                queue,
                shader_module,
                pipeline_layout,
                render_pipeline,
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::Resized(size) => {
                let App::Main { renderer, .. } = self else { return };
                renderer.surface_config.width = size.width;
                renderer.surface_config.height = size.height;
                renderer.should_resize_surface = true;
            },
            WindowEvent::RedrawRequested => {
                let App::Main { renderer, .. } = self else { return };
                
                if renderer.should_resize_surface {
                    renderer.surface.configure(&renderer.device, &renderer.surface_config);
                }
                
                let frame = renderer.surface.get_current_texture();
                
                match frame {
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
                            render_pass.draw(0..3, 0..1);
                        }
                        renderer.queue.submit(std::iter::once(commands.finish()));
                        frame.present();
                    }
                    Err(surface_error) => {
                        return;
                    }
                }
                
                
            },
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let App::Main { window, .. } = self else { return };
        window.request_redraw();
    }
    fn exiting(&mut self, event_loop: &ActiveEventLoop) {

        
    }
}


fn main() {
    let event_loop = EventLoop::new().unwrap();
    
    event_loop.run_app(&mut App::Init).unwrap()
}
