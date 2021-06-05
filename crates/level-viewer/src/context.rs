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
    pub depth_format: wgpu::TextureFormat,
    pub depth_texture: wgpu::Texture,
    pub depth_texture_view: wgpu::TextureView,
    pub aspect: f32,
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

        let size = window.inner_size();
        let (depth_texture, depth_texture_view) =
            Self::alloc_depth_texture(&device, size.width, size.height);

        Self {
            window,
            instance,
            surface,
            adapter,
            device,
            queue,
            format,
            depth_format: wgpu::TextureFormat::Depth32Float,
            depth_texture,
            depth_texture_view,
            aspect: 1.0,
        }
    }

    fn alloc_depth_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });
        (texture, texture_view)
    }

    fn resize(&mut self, renderer: &mut Renderer, width: u32, height: u32) {
        renderer
            .set_transform(self, {
                let eye = cgmath::point3(246.0, 310.0, 8.0);
                let aspect = width as f32 / height as f32;
                let proj = cgmath::perspective(cgmath::Deg(60.0), aspect, 1.0, 2000.0);
                let view = cgmath::Matrix4::look_to_rh(
                    eye,
                    cgmath::vec3(-1.0, -2.0, 0.0),
                    cgmath::Vector3::unit_z(),
                );
                proj * view
            })
            .unwrap();

        self.depth_texture.destroy();

        let (depth_texture, depth_texture_view) =
            Self::alloc_depth_texture(&self.device, width, height);
        self.depth_texture = depth_texture;
        self.depth_texture_view = depth_texture_view;
    }

    pub fn run(mut self, event_loop: EventLoop<()>, mut renderer: Renderer) {
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
        self.resize(&mut renderer, size.width, size.height);

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
                            self.resize(
                                &mut renderer,
                                swap_chain_descriptor.width,
                                swap_chain_descriptor.height,
                            );
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
