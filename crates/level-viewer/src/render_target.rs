use crate::context::Context;

pub struct SurfaceRenderTarget {
    swap_chain: wgpu::SwapChain,
    _depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    size: cgmath::Vector2<f32>,
}

impl SurfaceRenderTarget {
    pub fn new(context: &Context) -> Self {
        let winit::dpi::PhysicalSize { width, height } = context.window.inner_size();

        let swap_chain = context.device.create_swap_chain(
            &context.surface,
            &wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                format: context.format,
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
            },
        );

        let depth_texture = context.device.create_texture(&wgpu::TextureDescriptor {
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

        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor {
            aspect: wgpu::TextureAspect::DepthOnly,
            ..Default::default()
        });

        let size = cgmath::Vector2::new(width as f32, height as f32);

        Self {
            swap_chain,
            _depth_texture: depth_texture,
            depth_texture_view,
            size,
        }
    }

    pub fn size(&self) -> cgmath::Vector2<f32> {
        self.size
    }

    pub fn aspect(&self) -> f32 {
        self.size.x / self.size.y
    }

    pub fn depth_texture_view(&self) -> &wgpu::TextureView {
        &self.depth_texture_view
    }

    pub fn next_frame(&mut self, context: &Context) -> wgpu::SwapChainFrame {
        match self.swap_chain.get_current_frame() {
            Ok(frame) => frame,
            Err(_) => {
                *self = SurfaceRenderTarget::new(context);

                self.swap_chain
                    .get_current_frame()
                    .expect("next frame from swap chain after resize")
            }
        }
    }
}
