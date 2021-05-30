use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::renderer::Renderer;

pub struct Context {
    pub window: Window,
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
}

impl Context {
    pub async fn new(event_loop: &EventLoop<()>) -> Self {
        let window = WindowBuilder::new()
            .with_title("DF Level Viewer")
            .build(&event_loop)
            .unwrap();

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        // Safety: window is a valid window handle due to .unwrap()
        let surface = unsafe { instance.create_surface(&window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let format = adapter.get_swap_chain_preferred_format(&surface).unwrap();

        Self {
            window,
            instance,
            surface,
            adapter,
            device,
            queue,
            format,
        }
    }

    pub fn run(self, event_loop: EventLoop<()>, mut renderer: Renderer) {
        let size = self.window.inner_size();

        let mut swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: self.format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        let mut swap_chain = self
            .device
            .create_swap_chain(&self.surface, &swap_chain_descriptor);

        event_loop.run(move |event, _, control_flow| {
            use winit::event::{Event, WindowEvent};
            use winit::event_loop::ControlFlow;
            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent {
                    event: WindowEvent::Resized(size),
                    ..
                } => {
                    swap_chain_descriptor.width = size.width;
                    swap_chain_descriptor.height = size.height;
                }
                Event::RedrawRequested(_) => {
                    let frame = match swap_chain.get_current_frame() {
                        Err(_) => {
                            swap_chain = self
                                .device
                                .create_swap_chain(&self.surface, &swap_chain_descriptor);
                            swap_chain
                                .get_current_frame()
                                .expect("next frame from swap chain")
                        }
                        Ok(frame) => frame,
                    };

                    renderer.render(&self, &frame.output);
                }
                _ => {}
            }
        });
    }
}
