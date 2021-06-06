use crate::context::Context;
use crate::renderer::Renderer;

pub struct SurfaceRenderTarget {
    swap_chain: wgpu::SwapChain,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    aspect: f32,
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

        let aspect = width as f32 / height as f32;

        Self {
            swap_chain,
            depth_texture,
            depth_texture_view,
            aspect,
        }
    }

    pub fn render(&mut self, context: &Context, renderer: &mut Renderer) {
        let frame = match self.swap_chain.get_current_frame() {
            Ok(frame) => frame,
            Err(_) => {
                *self = SurfaceRenderTarget::new(context);

                self.swap_chain
                    .get_current_frame()
                    .expect("next frame from swap chain after resize")
            }
        };

        renderer.set_view(context, self.aspect);

        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachment {
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                    store: true,
                },
                view: &frame.output.view,
                resolve_target: None,
            }],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: false,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });

        renderer.render(&mut render_pass);
        drop(render_pass);

        context.queue.submit(std::iter::once(encoder.finish()));
    }
}
