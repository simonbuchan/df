use std::num::NonZeroU32;

use cgmath::prelude::*;

use crate::context::Context;
use crate::mesh::Vertex;

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    locals_buffer: wgpu::Buffer,
    level: crate::loader::Level,
    bind_group: wgpu::BindGroup,
}

macro_rules! shader_source {
    ($name: literal) => {
        wgpu::util::make_spirv(include_bytes!(concat!(env!("OUT_DIR"), "/", $name, ".spv")))
    };
}

impl Renderer {
    pub fn new(context: &Context, level: crate::loader::Level) -> Self {
        let vert_shader_module =
            context
                .device
                .create_shader_module(&wgpu::ShaderModuleDescriptor {
                    label: None,
                    flags: wgpu::ShaderFlags::empty(),
                    source: shader_source!("shader.vert.glsl"),
                });
        let frag_shader_module =
            context
                .device
                .create_shader_module(&wgpu::ShaderModuleDescriptor {
                    label: None,
                    flags: wgpu::ShaderFlags::empty(),
                    source: shader_source!("shader.frag.glsl"),
                });

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                min_binding_size: None,
                                has_dynamic_offset: false,
                            },
                            visibility: wgpu::ShaderStage::VERTEX_FRAGMENT,
                            binding: 0,
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            visibility: wgpu::ShaderStage::FRAGMENT,
                            binding: 1,
                            count: NonZeroU32::new(level.textures.len() as u32),
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

        let depth_format = wgpu::TextureFormat::Depth32Float;

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
                    entry_point: "main",
                    module: &vert_shader_module,
                    buffers: std::slice::from_ref(&Vertex::BUFFER_LAYOUT),
                },
                fragment: Some(wgpu::FragmentState {
                    entry_point: "main",
                    module: &frag_shader_module,
                    targets: &[wgpu::ColorTargetState {
                        format: context.format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::OVER,
                            alpha: wgpu::BlendComponent::OVER,
                        }),
                        write_mask: wgpu::ColorWrite::ALL,
                    }],
                }),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
            });

        let texture_views = level
            .textures
            .iter()
            .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
            .collect::<Vec<wgpu::TextureView>>();
        let texture_views = texture_views.iter().collect::<Vec<&wgpu::TextureView>>();

        let sampler = context.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            min_filter: wgpu::FilterMode::Nearest,
            mag_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let locals_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 64,
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::MAP_WRITE,
            mapped_at_creation: false,
        });

        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: locals_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureViewArray(&texture_views),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            });

        Self {
            pipeline,
            locals_buffer,
            level,
            bind_group,
        }
    }

    pub fn set_view(&mut self, context: &Context, aspect: f32) {
        self.set_transform(context, {
            let eye = cgmath::point3(246.0, 310.0, 8.0);
            let proj = cgmath::perspective(cgmath::Deg(45.0), aspect, 1.0, 2000.0);
            let view = cgmath::Matrix4::look_to_rh(
                eye,
                cgmath::vec3(-1.0, -2.0, 0.0),
                cgmath::Vector3::unit_z(),
            );
            proj * view
        })
        .unwrap();
    }

    pub fn set_transform(
        &mut self,
        context: &Context,
        transform: impl Into<cgmath::Matrix4<f32>>,
    ) -> Result<(), wgpu::BufferAsyncError> {
        fn imp(
            buffer: &wgpu::Buffer,
            context: &Context,
            transform: &[f32; 16],
        ) -> Result<(), wgpu::BufferAsyncError> {
            let slice = buffer.slice(..);
            let fut = slice.map_async(wgpu::MapMode::Write);
            crate::poll_device(&context, fut)?;
            slice
                .get_mapped_range_mut()
                .copy_from_slice(crate::transmute_slice(transform));
            buffer.unmap();
            Ok(())
        }

        imp(
            &self.locals_buffer,
            context,
            transform.into().transpose().as_ref(),
        )
    }

    pub fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);

        self.level.mesh.draw(render_pass);
    }
}
