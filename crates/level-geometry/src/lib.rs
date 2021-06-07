use formats::lev;

pub fn walls_to_polygons(walls: impl IntoIterator<Item = (usize, usize)>) -> Vec<Vec<usize>> {
    // open segment of a Polygon
    struct Contour {
        indices: std::collections::VecDeque<usize>,
    }

    impl Contour {
        fn new(a: usize, b: usize) -> Self {
            let mut indices = std::collections::VecDeque::with_capacity(8);
            indices.push_back(a);
            indices.push_back(b);
            Self { indices }
        }

        fn front(&self) -> usize {
            *self.indices.front().unwrap()
        }

        fn back(&self) -> usize {
            *self.indices.back().unwrap()
        }

        fn add(&mut self, a: usize, b: usize) -> Add {
            let add = self.add_near(a, b);
            match add {
                Add::Unmatched => self.add_near(b, a),
                add => return add,
            }
        }

        fn add_near(&mut self, near: usize, far: usize) -> Add {
            if near == self.back() {
                if far == self.front() {
                    Add::Closed
                } else {
                    self.indices.push_back(far);
                    Add::Extended
                }
            } else if near == self.front() {
                if far == self.back() {
                    Add::Closed
                } else {
                    self.indices.push_front(far);
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
                    polygons.push(polygon.into_iter().collect());
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

pub type TriangulationError = triangulate::TriangulationError<std::convert::Infallible>;

pub fn triangulate_sector(
    sector: &lev::Sector,
) -> Result<Vec<mint::Point2<f32>>, TriangulationError> {
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
//
// fn validate_lev(lev: &lev::Lev) {
//     for sector in &lev.sectors {
//         // In order to generate a triangulated floor / ceil, we want every vertex to be touched
//         // by the left and right of one wall each - but unfortunately
//         let mut left = (0..sector.vertices.len()).collect::<BTreeSet<_>>();
//         let mut right = (0..sector.vertices.len()).collect::<BTreeSet<_>>();
//         for wall in &sector.walls {
//             if !left.remove(&wall.left_vertex) {
//                 eprintln!(
//                     "sector {} vertex {} already used on left",
//                     sector.id, wall.left_vertex
//                 );
//             }
//             if !right.remove(&wall.right_vertex) {
//                 eprintln!(
//                     "sector {} vertex {} already used on right",
//                     sector.id, wall.right_vertex
//                 );
//             }
//         }
//         if !left.is_empty() && !right.is_empty() {
//             eprintln!(
//                 "sector {} has untouched left vertices: {:?}, right vertices: {:?}",
//                 sector.id, left, right,
//             );
//         }
//     }
// }
