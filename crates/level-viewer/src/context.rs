use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

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
                power_preference: wgpu::PowerPreference::HighPerformance,
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
                        // Jabba's ship has 300+ textures(!), but Intel UHD integrated
                        // graphics only supports 200 bound textures. We'll need a smarter
                        // renderer to support that, but for now require a reasonable amount.
                        // Seems 8k is what pretty much any dGPU will provide:
                        // https://vulkan.gpuinfo.org/displaydevicelimit.php?name=maxPerStageDescriptorSampledImages&platform=windows
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
