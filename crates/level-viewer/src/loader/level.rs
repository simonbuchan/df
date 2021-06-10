use cgmath::MetricSpace;

use formats::lev;

use crate::context::Context;
use crate::loader::{Loader, LoaderResult};
use crate::mesh::{Mesh, MeshBuilder, Vertex};
use level_geometry::triangulate_sector;

pub struct Level {
    pub textures: Vec<wgpu::Texture>,
    pub mesh: Mesh,
}

impl Level {
    pub fn load(loader: &mut Loader, name: &str, context: &Context) -> LoaderResult<Self> {
        let file = loader.dark.entry(name)?;
        let lev = lev::Lev::read(file)?;

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
                let left: cgmath::Point2<f32> = sector.vertices[wall.left_vertex].into();
                let right: cgmath::Point2<f32> = sector.vertices[wall.right_vertex].into();

                if let Some(adjoin_sector) = wall.adjoin_sector {
                    let adjoin_sector = &lev.sectors[adjoin_sector];
                    let adjoin_floor = adjoin_sector.floor_altitude;
                    let adjoin_ceil = adjoin_sector.ceiling_altitude;

                    if floor > adjoin_floor {
                        let texture = &wall.bottom_texture;
                        builder.add_wall(left, right, floor, adjoin_floor, texture, 0, light);
                    }

                    if ceil < adjoin_ceil {
                        let texture = &wall.top_texture;
                        builder.add_wall(left, right, adjoin_ceil, ceil, texture, 0, light);
                    }
                }

                if wall.adjoin_sector.is_none() || wall.flags.0 & 1 != 0 {
                    builder.add_wall(left, right, floor, ceil, &wall.middle_texture, 0, light);
                }
            }

            let triangulation = triangulate_sector(&sector);
            builder.add_floor(
                floor,
                &triangulation,
                &sector.floor_texture,
                if sector.flags.0 & 0x80 != 0 {
                    0x1_0000
                } else {
                    0
                },
                sector.ambient,
            );
            builder.add_ceil(
                ceil,
                &triangulation,
                &sector.ceiling_texture,
                if sector.flags.0 & 1 != 0 { 0x1_0000 } else { 0 },
                sector.ambient,
            );
        }

        let mesh = builder.build(context);

        Ok(Self { textures, mesh })
    }
}

struct LevelMeshBuilder {
    inner: MeshBuilder,
    texture_sizes: Vec<cgmath::Vector2<u32>>,
}

impl LevelMeshBuilder {
    fn new(texture_sizes: Vec<cgmath::Vector2<u32>>) -> Self {
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
        texture: &lev::Texture,
        texture_flags: u32,
        light: u32,
    ) {
        let index = match texture.index {
            None => return,
            Some(tex) => tex,
        };
        let texture_size = self.texture_sizes[index].cast::<f32>().unwrap();
        let tex = index as u32 | texture_flags;

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

    fn add_floor(
        &mut self,
        z: f32,
        triangulation: &[[mint::Point2<f32>; 3]],
        texture: &lev::Texture,
        texture_flags: u32,
        light: u32,
    ) {
        let tex = match texture.index {
            None => return,
            Some(tex) => tex as u32,
        } | texture_flags;

        for tri in triangulation {
            self.add_level_vertex(tri[0], z, tex, texture.offset, light);
            self.add_level_vertex(tri[1], z, tex, texture.offset, light);
            self.add_level_vertex(tri[2], z, tex, texture.offset, light);
        }
    }

    fn add_ceil(
        &mut self,
        z: f32,
        triangulation: &[[mint::Point2<f32>; 3]],
        texture: &lev::Texture,
        texture_flags: u32,
        light: u32,
    ) {
        let tex = match texture.index {
            None => return,
            Some(tex) => tex as u32,
        } | texture_flags;

        for tri in triangulation {
            self.add_level_vertex(tri[0], z, tex, texture.offset, light);
            self.add_level_vertex(tri[2], z, tex, texture.offset, light);
            self.add_level_vertex(tri[1], z, tex, texture.offset, light);
        }
    }

    fn add_level_vertex(
        &mut self,
        point: mint::Point2<f32>,
        z: f32,
        tex: u32,
        offset: mint::Vector2<f32>,
        light: u32,
    ) {
        self.inner.add(Vertex {
            pos: cgmath::point3(point.x, point.y, -z),
            uv: (cgmath::point2(point.x, point.y) - cgmath::Vector2::from(offset)) / 8.0,
            tex,
            light,
        });
    }

    fn build(self, context: &Context) -> Mesh {
        self.inner.build(context)
    }
}
