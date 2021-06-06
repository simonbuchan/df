use crate::context::Context;
use crate::render_target::SurfaceRenderTarget;
use crate::renderer::Renderer;

pub struct Camera {
    pub load: wgpu::LoadOp<wgpu::Color>,
    pub fov: cgmath::Rad<f32>,
    pub eye: cgmath::Point3<f32>,
    pub dir: cgmath::Rad<f32>,
}

impl Camera {
    pub fn new(eye: cgmath::Point3<f32>) -> Self {
        Self {
            load: wgpu::LoadOp::Clear(wgpu::Color::RED),
            fov: cgmath::Deg(45.0).into(),
            eye,
            dir: cgmath::Deg(180.0).into(),
        }
    }

    pub fn matrix(&self, aspect: f32) -> cgmath::Matrix4<f32> {
        let proj = cgmath::perspective(self.fov, aspect, 1.0, 2000.0);

        let (s, c) = cgmath::Angle::sin_cos(self.dir);
        let dir = cgmath::vec3(s, c, 0.0);

        let view = cgmath::Matrix4::look_to_rh(self.eye, dir, cgmath::Vector3::unit_z());

        proj * view
    }

    pub fn render(
        &mut self,
        context: &Context,
        target: &mut SurfaceRenderTarget,
        renderer: &mut Renderer,
    ) {
        let frame = target.next_frame(context);

        renderer
            .set_transform(context, self.matrix(target.aspect()))
            .unwrap();

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
                view: target.depth_texture_view(),
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
