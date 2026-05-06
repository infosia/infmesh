use std::ops::{Deref, DerefMut};

use nalgebra::Point3;

use crate::Scalar;
use crate::handle::{FaceHandle, VertexHandle};
use crate::error::MeshError;
use crate::mesh::Connectivity;

/// A triangle mesh with geometry (vertex positions).
///
/// Wraps `Connectivity` via `Deref` for transparent access to topology methods.
/// All faces are guaranteed to be triangles.
#[derive(Clone, Debug, Default)]
pub struct TriMesh {
    conn: Connectivity,
    points: Vec<Point3<Scalar>>,
}

impl Deref for TriMesh {
    type Target = Connectivity;
    fn deref(&self) -> &Connectivity {
        &self.conn
    }
}

impl DerefMut for TriMesh {
    fn deref_mut(&mut self) -> &mut Connectivity {
        &mut self.conn
    }
}

impl TriMesh {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a vertex with position.
    pub fn add_vertex(&mut self, point: Point3<Scalar>) -> VertexHandle {
        let vh = self.conn.new_vertex();
        self.points.push(point);
        vh
    }

    /// Get vertex position.
    pub fn point(&self, vh: VertexHandle) -> &Point3<Scalar> {
        &self.points[vh.idx()]
    }

    /// Get mutable vertex position.
    pub fn point_mut(&mut self, vh: VertexHandle) -> &mut Point3<Scalar> {
        &mut self.points[vh.idx()]
    }

    /// Add a triangular face. If more than 3 vertices are given, auto-triangulates
    /// by fan triangulation from vertex 0.
    pub fn add_face(&mut self, vertices: &[VertexHandle]) -> Result<FaceHandle, MeshError> {
        if vertices.len() < 3 {
            return Err(MeshError::DegenerateFace);
        }
        if vertices.len() == 3 {
            return self.conn.add_face(vertices);
        }
        // Fan triangulation: v[0], v[i], v[i+1] for i in 1..n-1
        let mut last_fh = self.conn.add_face(&[vertices[0], vertices[1], vertices[2]])?;
        for i in 2..vertices.len() - 1 {
            last_fh = self.conn.add_face(&[vertices[0], vertices[i], vertices[i + 1]])?;
        }
        Ok(last_fh)
    }

    /// Compact arrays by removing deleted elements. Also compacts the
    /// `points` vector to stay in sync with the vertex indices.
    pub fn garbage_collection(&mut self) {
        let v_map = self.conn.garbage_collection();
        let mut new_vi = 0usize;
        for (i, v) in v_map.iter().enumerate() {
            if v.idx() != u32::MAX as usize {
                if new_vi != i {
                    self.points[new_vi] = self.points[i];
                }
                new_vi += 1;
            }
        }
        self.points.truncate(new_vi);
    }

    /// Get the connectivity (non-deref access).
    pub fn connectivity(&self) -> &Connectivity {
        &self.conn
    }

    /// Get mutable connectivity.
    pub fn connectivity_mut(&mut self) -> &mut Connectivity {
        &mut self.conn
    }
}
