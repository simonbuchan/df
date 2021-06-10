use formats::lev;

pub fn walls_to_polygons(walls: impl IntoIterator<Item = (usize, usize)>) -> Vec<Vec<usize>> {
    // open segment of a Polygon
    struct Contour {
        indices: std::collections::VecDeque<usize>,
    }

    impl Contour {
        fn new(left: usize, right: usize) -> Self {
            let mut indices = std::collections::VecDeque::with_capacity(8);
            indices.push_back(left);
            indices.push_back(right);
            Self { indices }
        }

        fn left_end(&self) -> usize {
            *self.indices.front().unwrap()
        }

        fn right_end(&self) -> usize {
            *self.indices.back().unwrap()
        }

        fn add(&mut self, left: usize, right: usize, dup: bool) -> Add {
            if left == self.right_end() {
                if right == self.left_end() {
                    Add::Closed
                } else if !dup {
                    self.indices.push_back(right);
                    Add::Extended
                } else {
                    Add::Unmatched
                }
            } else if right == self.left_end() {
                if left == self.right_end() {
                    Add::Closed
                } else if !dup {
                    self.indices.push_front(left);
                    Add::Extended
                } else {
                    Add::Unmatched
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
    let mut added = std::collections::BTreeSet::new();

    'wall: for (left, right) in walls {
        // detect if this is a dup wall, and discard if so.
        // still need the last wall to close an inner polygon though.
        let dup = !added.insert(left) & !added.insert(right);

        for i in 0..contours.len() {
            let add = contours[i].add(left, right, dup);
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

        if !dup {
            contours.push(Contour::new(left, right));
        }
    }

    polygons.sort_unstable_by_key(|p| p[0]);

    polygons
}

pub fn triangulate_sector(sector: &lev::Sector) -> Vec<[mint::Point2<f32>; 3]> {
    // reverse the polygon order by flipping the wall ends as Seidel requires it.
    let polygons = walls_to_polygons(sector.walls.iter().map(|w| (w.right_vertex, w.left_vertex)));

    let contours = polygons.iter().map(|p| p.len() as i32).collect::<Vec<_>>();
    let vertices = polygons
        .iter()
        .flatten()
        .map(|&i| {
            let vertex = sector.vertices[i];
            seidel::Vertex {
                x: vertex.x as f64,
                y: vertex.y as f64,
            }
        })
        .collect::<Vec<_>>();

    let triangles = seidel::triangulate(&contours, &vertices);
    fn v(vertex: seidel::Vertex) -> mint::Point2<f32> {
        mint::Point2 {
            x: vertex.x as f32,
            y: vertex.y as f32,
        }
    }
    triangles
        .into_iter()
        .map(|t| {
            [
                v(vertices[t.0[0] as usize - 1]),
                v(vertices[t.0[1] as usize - 1]),
                v(vertices[t.0[2] as usize - 1]),
            ]
        })
        .collect()
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
