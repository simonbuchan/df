#![allow(dead_code)]
#![warn(rust_2018_idioms)]

use std::fmt;
use std::os::raw::{c_double, c_int};

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Vertex {
    pub x: c_double,
    pub y: c_double,
}

impl From<[c_double; 2]> for Vertex {
    fn from([x, y]: [c_double; 2]) -> Self {
        Self { x, y }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Triangle(pub [c_int; 3]);

impl fmt::Debug for Triangle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

static LOCK_INIT: std::sync::Once = std::sync::Once::new();
static mut LOCK: Option<std::sync::Mutex<()>> = None;

#[link(name = "seidel-triangulate")]
extern "C" {
    fn triangulate_polygon(
        ncontours: c_int,
        cntr: *const c_int,
        vertices: *const Vertex,
        triangles: *mut Triangle,
    ) -> c_int;
    // fn is_point_inside_polygon(vertex: Vertex) -> c_int;
}

pub fn triangulate(contours: &[c_int], vertices: &[Vertex]) -> Vec<Triangle> {
    if contours.is_empty() {
        return vec![];
    }

    LOCK_INIT.call_once(|| {
        unsafe { LOCK = Some(Default::default()) };
    });
    let _guard = unsafe { LOCK.as_mut().unwrap() }.lock().unwrap();

    let triangle_count = vertices.len() - 2 + 2 * (contours.len() - 1);
    let mut result = vec![Triangle([0, 0, 0]); triangle_count];
    unsafe {
        triangulate_polygon(
            contours.len() as c_int,
            contours.as_ptr(),
            vertices.as_ptr().sub(1), // ignores, and doesn't access vertex at 0
            result.as_mut_ptr(),
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn it_works() {
        const CONTOURS: [c_int; 4] = [4, 3, 3, 3];

        #[rustfmt::skip]
        let vertices: &[Vertex] = &[
            [0.0, 0.0].into(), [6.0, 0.0].into(), [6.0, 6.0].into(), [0.0, 6.0].into(),

            [0.5, 1.0].into(), [1.0, 2.0].into(), [2.0, 1.5].into(),

            [0.5, 4.0].into(), [1.0, 5.0].into(), [2.0, 4.5].into(),

            [3.0, 3.0].into(), [5.0, 3.5].into(), [5.0, 2.5].into(),
        ];

        let triangles = triangulate(&CONTOURS, vertices);
        println!("{:?}", triangles);
    }

    #[test]
    fn test_2() {
        let contours = &[4];
        let vertices = &[
            Vertex {
                x: 252.0,
                y: -224.0,
            },
            Vertex {
                x: 236.0,
                y: -224.0,
            },
            Vertex {
                x: 236.0,
                y: -228.0,
            },
            Vertex {
                x: 252.0,
                y: -228.0,
            },
        ];
        let triangles = triangulate(contours, vertices);
        println!("{:?}", triangles);
    }
}
