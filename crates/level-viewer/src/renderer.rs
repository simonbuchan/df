use crate::context::Context;

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone)]
struct Vertex {
    pos: [f32; 4],
}

impl Vertex {
    const STRIDE: wgpu::BufferAddress = std::mem::size_of::<Self>() as _;

    const fn new(x: f32, y: f32) -> Self {
        Self {
            pos: [x, y, 0.0, 1.0],
        }
    }
}

struct Mesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

impl Mesh {
    fn triangle(context: &Context) -> Self {
        const VERTICES: &[Vertex] = &[
            Vertex::new(0.0, 0.5),
            Vertex::new(-0.5, -0.5),
            Vertex::new(0.5, -0.5),
        ];
        const INDICES: &[u16] = &[0, 1, 2];
        Self::new(context, VERTICES, INDICES)
    }

    fn transmute_slice<T: Copy, U: Copy>(src: &[T]) -> &[U] {
        let size = std::mem::size_of_val(src);
        let rem = size % std::mem::size_of::<U>();
        assert_eq!(rem, 0);
        let len = size / std::mem::size_of::<U>();
        let ptr = src.as_ptr();
        // Safety: src and dst contents are Copy, we have checked sizes above
        unsafe { std::slice::from_raw_parts(ptr.cast(), len) }
    }

    fn new(context: &Context, vertex_data: &[Vertex], index_data: &[u16]) -> Self {
        let vertices = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsage::VERTEX,
                contents: Self::transmute_slice(vertex_data),
            });

        let indices = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsage::INDEX,
                contents: Self::transmute_slice(index_data),
            });

        Self {
            vertices,
            indices,
            index_count: index_data.len() as u32,
        }
    }

    fn draw<'a>(&'a self, encoder: &mut wgpu::RenderPass<'a>) {
        encoder.set_vertex_buffer(0, self.vertices.slice(..));
        encoder.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
        encoder.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    mesh: Mesh,
}

impl Renderer {
    pub fn new(context: &Context) -> Self {
        let shader_module = context
            .device
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: None,
                flags: wgpu::ShaderFlags::empty(),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: None, // Some(&pipeline_layout),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    ..Default::default()
                },
                vertex: wgpu::VertexState {
                    entry_point: "vs_main",
                    module: &shader_module,
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: Vertex::STRIDE,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &[wgpu::VertexAttribute {
                            offset: 0,
                            format: wgpu::VertexFormat::Float32x4,
                            shader_location: 0,
                        }],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    entry_point: "fs_main",
                    module: &shader_module,
                    targets: &[context.format.into()],
                }),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
            });

        let mesh = Mesh::triangle(context);

        Self { pipeline, mesh }
    }

    pub fn render(&mut self, context: &Context, frame: &wgpu::SwapChainTexture) {
        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachment {
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::RED),
                    store: true,
                },
                view: &frame.view,
                resolve_target: None,
            }],
            ..Default::default()
        });

        render_pass.set_pipeline(&self.pipeline);

        self.mesh.draw(&mut render_pass);

        drop(render_pass);

        context.queue.submit(std::iter::once(encoder.finish()));
    }
}
