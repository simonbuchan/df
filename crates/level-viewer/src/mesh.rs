use crate::context::Context;
use std::ops::Range;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Vertex {
    pub pos: cgmath::Point3<f32>,
    pub uv: cgmath::Point2<f32>,
    pub tex: u32,
    pub light: u32,
}

impl Vertex {
    pub const STRIDE: wgpu::BufferAddress = std::mem::size_of::<Self>() as _;

    pub const BUFFER_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: Self::STRIDE,
        step_mode: wgpu::InputStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                format: wgpu::VertexFormat::Float32x3,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                offset: 4 * 3,
                format: wgpu::VertexFormat::Float32x2,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                offset: 4 * 5,
                format: wgpu::VertexFormat::Uint32,
                shader_location: 2,
            },
            wgpu::VertexAttribute {
                offset: 4 * 6,
                format: wgpu::VertexFormat::Uint32,
                shader_location: 3,
            },
        ],
    };
}

#[derive(Default)]
pub struct MeshBuilder {
    vertex_data: Vec<Vertex>,
    index_data: Vec<u16>,
}

impl MeshBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tri(&mut self, vertices: &[Vertex; 3]) {
        let index = self.vertex_data.len() as u16;
        self.vertex_data.extend_from_slice(vertices);
        self.index_data.extend(index..index + 3);
    }

    pub fn quad(&mut self, vertices: &[Vertex; 4]) -> Range<usize> {
        let index = self.vertex_data.len() as u16;
        self.vertex_data.extend_from_slice(vertices);
        const OFFSETS: &'static [u16] = &[0, 1, 2, 2, 1, 3];
        let index_start = self.index_data.len();
        self.index_data.extend(OFFSETS.iter().map(|&o| index + o));
        index_start..self.index_data.len()
    }

    pub fn build(&self, context: &Context) -> Mesh {
        Mesh::new(context, &self.vertex_data, &self.index_data)
    }
}

pub struct Mesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

impl Mesh {
    pub fn new(context: &Context, vertex_data: &[Vertex], index_data: &[u16]) -> Self {
        let vertices = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsage::VERTEX,
                contents: crate::transmute_slice(vertex_data),
            });

        let indices = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsage::INDEX,
                contents: crate::transmute_slice(index_data),
            });

        Self {
            vertices,
            indices,
            index_count: index_data.len() as u32,
        }
    }

    pub fn set<'a>(&'a self, encoder: &mut wgpu::RenderPass<'a>) {
        encoder.set_vertex_buffer(0, self.vertices.slice(..));
        encoder.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint16);
    }

    pub fn draw<'a>(&'a self, encoder: &mut wgpu::RenderPass<'a>) {
        self.set(encoder);
        encoder.draw_indexed(0..self.index_count, 0, 0..1);
    }
}
