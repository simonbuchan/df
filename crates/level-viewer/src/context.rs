use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

use crate::render_target::SurfaceRenderTarget;
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
                    features: wgpu::Features::MAPPABLE_PRIMARY_BUFFERS
                        | wgpu::Features::SAMPLED_TEXTURE_BINDING_ARRAY
                        | wgpu::Features::SAMPLED_TEXTURE_ARRAY_NON_UNIFORM_INDEXING
                        | wgpu::Features::UNSIZED_BINDING_ARRAY,
                    limits: wgpu::Limits {
                        // Jabba's ship has 300+ textures(!), but my RTX 2070S claims 1M, so...
                        max_sampled_textures_per_shader_stage: 1024,
                        ..Default::default()
                    },
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
}
