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
    // texture: wgpu::Texture,
    // texture_view: wgpu::TextureView,
    // sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub fn new(context: &Context, texture: wgpu::Texture) -> Self {
        let shader_module = context
            .device
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: None,
                flags: wgpu::ShaderFlags::empty(),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            });

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            binding: 1,
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            ty: wgpu::BindingType::Sampler {
                                comparison: false,
                                filtering: true,
                            },
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            binding: 2,
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
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
                    targets: &[wgpu::ColorTargetState {
                        format: context.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::OVER,
                            alpha: wgpu::BlendComponent::OVER,
                        }),
                        write_mask: wgpu::ColorWrite::ALL,
                    }],
                }),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
            });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            ..Default::default()
        });

        let sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            min_filter: wgpu::FilterMode::Nearest,
            mag_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

        let mesh = Mesh::triangle(context);

        Self {
            pipeline,
            mesh,
            // texture,
            // texture_view,
            // sampler,
            bind_group,
        }
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
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        self.mesh.draw(&mut render_pass);

        drop(render_pass);

        context.queue.submit(std::iter::once(encoder.finish()));
    }
}
