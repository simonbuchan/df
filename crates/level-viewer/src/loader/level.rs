use std::ops::Range;

use formats::common::{Vec2, Vec2f32, Vec2u16};

use crate::context::Context;
use crate::loader::{Loader, LoaderResult};
use crate::mesh::{Mesh, MeshBuilder, Vertex};
use cgmath::MetricSpace;
use std::collections::BTreeSet;

pub struct Level {
    pub textures: Vec<wgpu::Texture>,
    pub mesh: Mesh,
}

impl Level {
    pub fn load(loader: &mut Loader, name: &str, context: &Context) -> LoaderResult<Self> {
        let file = loader.dark.entry(name)?;
        let lev = formats::lev::Lev::read(file)?;

        validate_lev(&lev);

        let pal = loader.load_pal(&lev.palette_name)?;

        let (sizes, textures) = lev
            .texture_names
            .iter()
            .map(|name| loader.load_bm_or_default(name, &pal, context))
            .unzip();

        let mut builder = LevelMeshBuilder::new(sizes);

        for sector in &lev.sectors {
            let floor = sector.floor_altitude;
            let ceil = sector.ceiling_altitude;
            for wall in &sector.walls {
                let light = (sector.ambient as i32 + wall.light as i32) as u32;
                let left = vec2_to_point(sector.vertices[wall.left_vertex]);
                let right = vec2_to_point(sector.vertices[wall.right_vertex]);
                if let Some(adjoin_sector) = wall.adjoin_sector {
                    let adjoin_sector = &lev.sectors[adjoin_sector];
                    let adjoin_floor = adjoin_sector.floor_altitude;
                    let adjoin_ceil = adjoin_sector.ceiling_altitude;

                    if floor > adjoin_floor {
                        let texture = &wall.bottom_texture;
                        builder.add_wall(left, right, floor, adjoin_floor, texture, light);
                    }

                    if ceil < adjoin_ceil {
                        let texture = &wall.top_texture;
                        builder.add_wall(left, right, adjoin_ceil, ceil, texture, light);
                    }
                } else {
                    builder.add_wall(left, right, floor, ceil, &wall.middle_texture, light);
                }
            }
        }

        let mesh = builder.build(context);

        Ok(Self { textures, mesh })
    }
}

fn validate_lev(lev: &formats::lev::Lev) {
    for sector in &lev.sectors {
        // In order to generate a triangulated floor / ceil, we want every vertex to be touched
        // by the left and right of one wall each - but unfortunately
        let mut left = (0..sector.vertices.len()).collect::<BTreeSet<_>>();
        let mut right = (0..sector.vertices.len()).collect::<BTreeSet<_>>();
        for wall in &sector.walls {
            if !left.remove(&wall.left_vertex) {
                eprintln!(
                    "sector {} vertex {} already used on left",
                    sector.id, wall.left_vertex
                );
            }
            if !right.remove(&wall.right_vertex) {
                eprintln!(
                    "sector {} vertex {} already used on right",
                    sector.id, wall.right_vertex
                );
            }
        }
        if !left.is_empty() && !right.is_empty() {
            eprintln!(
                "sector {} has untouched left vertices: {:?}, right vertices: {:?}",
                sector.id, left, right,
            );
        }
    }
}

fn vec2_to_point<T>(value: Vec2<T>) -> cgmath::Point2<T> {
    cgmath::point2(value.x, value.y)
}

struct LevelMeshBuilder {
    inner: MeshBuilder,
    texture_sizes: Vec<Vec2u16>,
}

impl LevelMeshBuilder {
    fn new(texture_sizes: Vec<Vec2u16>) -> Self {
        Self {
            texture_sizes,
            inner: MeshBuilder::new(),
        }
    }

    fn add_wall(
        &mut self,
        left: cgmath::Point2<f32>,
        right: cgmath::Point2<f32>,
        floor: f32,
        ceil: f32,
        texture: &formats::lev::Texture,
        light: u32,
    ) {
        if let Some(tex) = texture.index {
            let texture_size = self.texture_sizes[tex].into_vec2::<f32>();
            let tex = tex as u32;

            // A 64-texel wide texture maps exactly to 8.0 map units,
            // So the UV for a width of 8.0 should be mapped to 0.0 to 1.0
            let scale = 8.0 / cgmath::vec2(texture_size.x, texture_size.y);
            let width = left.distance(right) * scale.x;
            let height = (ceil - floor) * scale.y;
            let offset = cgmath::vec2(texture.offset.x * scale.x, -texture.offset.y * scale.y);

            self.inner.quad(&[
                Vertex {
                    pos: cgmath::point3(left.x, left.y, -floor),
                    uv: cgmath::point2(0.0, 0.0) + offset,
                    tex,
                    light,
                },
                Vertex {
                    pos: cgmath::point3(right.x, right.y, -floor),
                    uv: cgmath::point2(width, 0.0) + offset,
                    tex,
                    light,
                },
                Vertex {
                    pos: cgmath::point3(left.x, left.y, -ceil),
                    uv: cgmath::point2(0.0, height) + offset,
                    tex,
                    light,
                },
                Vertex {
                    pos: cgmath::point3(right.x, right.y, -ceil),
                    uv: cgmath::point2(width, height) + offset,
                    tex,
                    light,
                },
            ]);
        }
    }

    fn build(self, context: &Context) -> Mesh {
        self.inner.build(context)
    }
}
