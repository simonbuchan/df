use std::collections::BTreeSet;

use cgmath::MetricSpace;

use formats::lev;

use crate::context::Context;
use crate::loader::{Loader, LoaderResult};
use crate::mesh::{Mesh, MeshBuilder, Vertex};

pub struct Level {
    pub textures: Vec<wgpu::Texture>,
    pub mesh: Mesh,
}

impl Level {
    pub fn load(loader: &mut Loader, name: &str, context: &Context) -> LoaderResult<Self> {
        let file = loader.dark.entry(name)?;
        let lev = lev::Lev::read(file)?;

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
                let left: cgmath::Point2<f32> = sector.vertices[wall.left_vertex].into();
                let right: cgmath::Point2<f32> = sector.vertices[wall.right_vertex].into();
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

            match triangulate_sector(&sector) {
                Err(error) => {
                    eprintln!("sector {} triangulation error: {}", sector.id, error,);
                }
                Ok(triangulation) => {
                    builder.add_level(floor, &triangulation, &sector.floor_texture, sector.ambient);
                    builder.add_level(
                        ceil,
                        &triangulation,
                        &sector.ceiling_texture,
                        sector.ambient,
                    );
                }
            }
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
        light: u32,
    ) {
        if let Some(tex) = texture.index {
            let texture_size = self.texture_sizes[tex].cast::<f32>().unwrap();
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

    fn add_level(
        &mut self,
        z: f32,
        triangulation: &[mint::Point2<f32>],
        texture: &lev::Texture,
        light: u32,
    ) {
        let index = match texture.index {
            None => return,
            Some(index) => index,
        };
        for tri in triangulation.chunks_exact(3) {
            self.inner.tri(&[
                Vertex {
                    pos: cgmath::point3(tri[0].x, tri[0].y, -z),
                    uv: cgmath::point2(tri[0].x, tri[0].y) / 8.0,
                    tex: index as u32,
                    light,
                },
                Vertex {
                    pos: cgmath::point3(tri[1].x, tri[1].y, -z),
                    uv: cgmath::point2(tri[1].x, tri[1].y) / 8.0,
                    tex: index as u32,
                    light,
                },
                Vertex {
                    pos: cgmath::point3(tri[2].x, tri[2].y, -z),
                    uv: cgmath::point2(tri[2].x, tri[2].y) / 8.0,
                    tex: index as u32,
                    light,
                },
            ])
        }
    }

    fn build(self, context: &Context) -> Mesh {
        self.inner.build(context)
    }
}

fn walls_to_polygons(walls: impl IntoIterator<Item = (usize, usize)>) -> Vec<Vec<usize>> {
    // open segment of a Polygon
    struct Contour {
        indices: Vec<usize>,
    }

    impl Contour {
        fn new(a: usize, b: usize) -> Self {
            Self {
                indices: vec![a, b],
            }
        }

        fn first(&self) -> usize {
            self.indices[0]
        }

        fn last(&self) -> usize {
            self.indices[self.indices.len() - 1]
        }

        fn add(&mut self, a: usize, b: usize) -> Add {
            let add = self.add_near(a, b);
            match add {
                Add::Unmatched => self.add_near(b, a),
                add => return add,
            }
        }

        fn add_near(&mut self, near: usize, far: usize) -> Add {
            if near == self.last() {
                if far == self.first() {
                    Add::Closed
                } else {
                    self.indices.push(far);
                    Add::Extended
                }
            } else if near == self.first() {
                if far == self.last() {
                    Add::Closed
                } else {
                    self.indices.insert(0, far);
                    Add::Extended
                }
            } else {
                Add::Unmatched
            }
        }
    }

    enum Add {
        Unmatched,
        Closed,
        Extended,
    }

    let mut polygons: Vec<Vec<usize>> = Vec::new();
    let mut contours: Vec<Contour> = Vec::new();

    'wall: for (a, b) in walls {
        for i in 0..contours.len() {
            let add = contours[i].add(a, b);
            match add {
                Add::Unmatched => {}
                Add::Extended => continue 'wall,
                Add::Closed => {
                    let mut polygon = contours.remove(i).indices;
                    let min_index = polygon.iter().enumerate().min_by_key(|x| x.1).unwrap().0;
                    polygon.rotate_left(min_index);
                    polygons.push(polygon);
                    continue 'wall;
                }
            }
        }

        contours.push(Contour::new(a, b));
    }

    polygons.sort_unstable_by_key(|p| p[0]);

    polygons
}

#[test]
fn walls_to_polygons_test() {
    assert_eq!(
        walls_to_polygons(vec![(0, 1), (1, 2), (2, 0)]),
        vec![vec![0, 1, 2]],
    );
    assert_eq!(
        walls_to_polygons(vec![(0, 1), (1, 2), (2, 3), (3, 0)]),
        vec![vec![0, 1, 2, 3]],
    );
    assert_eq!(
        walls_to_polygons(vec![(0, 1), (1, 3), (2, 3), (2, 0)]),
        vec![vec![0, 1, 3, 2]],
    );
    // denormalization
    assert_eq!(
        walls_to_polygons(vec![(1, 2), (0, 1), (2, 0)]),
        vec![vec![0, 1, 2]],
    );
    assert_eq!(
        walls_to_polygons(vec![(1, 2), (1, 0), (2, 0)]),
        vec![vec![0, 1, 2]],
    );

    assert_eq!(
        walls_to_polygons(vec![(1, 2), (0, 1), (3, 4), (2, 0), (6, 4), (3, 6)]),
        vec![vec![0, 1, 2], vec![3, 4, 6]],
    );
}

type TriangulationError = triangulate::TriangulationError<std::convert::Infallible>;

fn triangulate_sector(sector: &lev::Sector) -> Result<Vec<mint::Point2<f32>>, TriangulationError> {
    let polygons = walls_to_polygons(sector.walls.iter().map(|w| (w.left_vertex, w.right_vertex)));

    #[derive(Copy, Clone, Debug)]
    struct Vertex(mint::Vector2<f32>);

    impl triangulate::Vertex for Vertex {
        type Coordinate = f32;
        fn x(&self) -> f32 {
            self.0.x
        }
        fn y(&self) -> f32 {
            self.0.y
        }
    }

    use triangulate::{
        builders::{FanToListAdapter, VecListBuilder},
        IndexWithU16U16, Triangulate,
    };

    let polygons: Vec<Vec<Vertex>> = polygons
        .into_iter()
        .map(|polygon| {
            polygon
                .into_iter()
                .map(|index| Vertex(sector.vertices[index].into()))
                .collect()
        })
        .collect();

    let polygon_list = IndexWithU16U16::new(&polygons);
    let mut vertices = Vec::new();
    polygon_list.triangulate::<FanToListAdapter<_, VecListBuilder<_>>>(&mut vertices)?;
    Ok(vertices.into_iter().map(|vertex| vertex.0.into()).collect())
}

fn validate_lev(lev: &lev::Lev) {
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
