mod renderer;

use std::sync::Arc;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop};
use winit::event_loop::EventLoop;
use winit::window::WindowId;
use winit::window::Window;

use renderer::Renderer;

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
        let renderer = Renderer::initialize(&window);

        *self = App::Main {
            window,
            renderer,
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::Resized(size) => {
                let App::Main { renderer, window } = self else { return };
                if size.width != 0 && size.height != 0 {
                    renderer.resize_surface(size);
                    window.request_redraw();
                }
            },
            WindowEvent::RedrawRequested => {
                let App::Main { renderer, .. } = self else { return };

                renderer.draw();
            },
            _ => {}
        }
    }
    
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let App::Main { window, .. } = self else { return };
        window.request_redraw();
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::Init;
    
    event_loop.run_app(&mut app).unwrap();
}
