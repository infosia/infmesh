use nalgebra::{Point3, Vector3};

use crate::Scalar;
use crate::handle::{EdgeHandle, FaceHandle, HalfedgeHandle, VertexHandle};
use crate::trimesh::TriMesh;
use crate::polymesh::PolyMesh;

/// Trait for meshes that have geometry (vertex positions).
pub trait MeshGeometry {
    fn conn(&self) -> &crate::mesh::Connectivity;
    fn point(&self, vh: VertexHandle) -> &Point3<Scalar>;
}

impl MeshGeometry for TriMesh {
    fn conn(&self) -> &crate::mesh::Connectivity { &**self }
    fn point(&self, vh: VertexHandle) -> &Point3<Scalar> { self.point(vh) }
}

impl MeshGeometry for PolyMesh {
    fn conn(&self) -> &crate::mesh::Connectivity { &**self }
    fn point(&self, vh: VertexHandle) -> &Point3<Scalar> { self.point(vh) }
}

/// Geometry calculations that work on any mesh with positions.
pub fn calc_edge_vector<M: MeshGeometry>(mesh: &M, hh: HalfedgeHandle) -> Vector3<Scalar> {
    let conn = mesh.conn();
    let p0 = mesh.point(conn.from_vertex_handle(hh));
    let p1 = mesh.point(conn.to_vertex_handle(hh));
    p1 - p0
}

pub fn calc_edge_length<M: MeshGeometry>(mesh: &M, eh: EdgeHandle) -> Scalar {
    calc_edge_vector(mesh, eh.h0()).norm()
}

pub fn calc_edge_sqr_length<M: MeshGeometry>(mesh: &M, eh: EdgeHandle) -> Scalar {
    calc_edge_vector(mesh, eh.h0()).norm_squared()
}

/// Calculate the normal of a triangle face from three points.
pub fn calc_face_normal_pts(p0: &Point3<Scalar>, p1: &Point3<Scalar>, p2: &Point3<Scalar>) -> Vector3<Scalar> {
    let d0 = p1 - p0;
    let d1 = p2 - p0;
    d0.cross(&d1)
}

/// Calculate the normal of a face using Newell's method (works for polygons).
pub fn calc_face_normal<M: MeshGeometry>(mesh: &M, fh: FaceHandle) -> Vector3<Scalar> {
    let conn = mesh.conn();
    let start = conn.face_halfedge_handle(fh);

    // Collect vertices of the face
    let mut vertices = Vec::new();
    let mut hh = start;
    loop {
        vertices.push(conn.to_vertex_handle(hh));
        hh = conn.next_halfedge_handle(hh);
        if hh == start {
            break;
        }
    }

    if vertices.len() == 3 {
        // Triangle: simple cross product
        let p0 = mesh.point(vertices[0]);
        let p1 = mesh.point(vertices[1]);
        let p2 = mesh.point(vertices[2]);
        let n = calc_face_normal_pts(p0, p1, p2);
        let len = n.norm();
        if len > 0.0 { n / len } else { Vector3::zeros() }
    } else {
        // Polygon: Newell's method
        let n = vertices.len();
        let mut normal: Vector3<Scalar> = Vector3::zeros();
        for i in 0..n {
            let p0 = mesh.point(vertices[i]);
            let p1 = mesh.point(vertices[(i + 1) % n]);
            normal.x += (p0.y - p1.y) * (p0.z + p1.z);
            normal.y += (p0.z - p1.z) * (p0.x + p1.x);
            normal.z += (p0.x - p1.x) * (p0.y + p1.y);
        }
        let len = normal.norm();
        if len > 0.0 { normal / len } else { Vector3::zeros() }
    }
}

/// Calculate vertex normal as the average of adjacent face normals.
///
/// Each face normal is normalized before summing, so all faces contribute equally
/// regardless of area. This matches the C++ OpenMesh default behavior. For
/// area-weighted normals, sum unnormalized cross products instead.
pub fn calc_vertex_normal<M: MeshGeometry>(mesh: &M, vh: VertexHandle) -> Vector3<Scalar> {
    let conn = mesh.conn();
    let mut normal = Vector3::zeros();

    for fh in conn.vf_cw_iter(vh) {
        normal += calc_face_normal(mesh, fh);
    }

    let len = normal.norm();
    if len > 0.0 { normal / len } else { Vector3::zeros() }
}

/// Calculate the centroid (average position) of a face.
pub fn calc_face_centroid<M: MeshGeometry>(mesh: &M, fh: FaceHandle) -> Point3<Scalar> {
    let conn = mesh.conn();
    let start = conn.face_halfedge_handle(fh);
    let mut sum = Vector3::zeros();
    let mut count = 0;

    let mut hh = start;
    loop {
        sum += mesh.point(conn.to_vertex_handle(hh)).coords;
        count += 1;
        hh = conn.next_halfedge_handle(hh);
        if hh == start {
            break;
        }
    }

    Point3::from(sum / count as Scalar)
}

/// Calculate the midpoint of an edge.
pub fn calc_edge_midpoint<M: MeshGeometry>(mesh: &M, eh: EdgeHandle) -> Point3<Scalar> {
    let conn = mesh.conn();
    let h0 = eh.h0();
    let p0 = mesh.point(conn.from_vertex_handle(h0));
    let p1 = mesh.point(conn.to_vertex_handle(h0));
    Point3::from((p0.coords + p1.coords) / 2.0)
}

/// Calculate the centroid of the entire mesh (average of all vertex positions).
pub fn calc_mesh_centroid<M: MeshGeometry>(mesh: &M) -> Point3<Scalar> {
    let conn = mesh.conn();
    let mut sum = Vector3::zeros();
    let mut count = 0;

    for vh in conn.vertices() {
        sum += mesh.point(vh).coords;
        count += 1;
    }

    if count > 0 {
        Point3::from(sum / count as Scalar)
    } else {
        Point3::origin()
    }
}

/// Calculate the area of a triangular face.
///
/// Only valid for triangular faces — uses exactly three vertices.
/// For polygon faces, triangulate first or use a polygon area method.
pub fn calc_face_area<M: MeshGeometry>(mesh: &M, fh: FaceHandle) -> Scalar {
    let conn = mesh.conn();
    let start = conn.face_halfedge_handle(fh);
    let v0 = conn.to_vertex_handle(start);
    let v1 = conn.to_vertex_handle(conn.next_halfedge_handle(start));
    let v2 = conn.to_vertex_handle(conn.next_halfedge_handle(conn.next_halfedge_handle(start)));

    let p0 = mesh.point(v0);
    let p1 = mesh.point(v1);
    let p2 = mesh.point(v2);

    calc_face_normal_pts(p0, p1, p2).norm() / 2.0
}

/// Calculate the dihedral angle at an edge (angle between the two adjacent face normals).
/// Returns 0.0 for boundary edges.
pub fn calc_dihedral_angle<M: MeshGeometry>(mesh: &M, eh: EdgeHandle) -> Scalar {
    let conn = mesh.conn();
    let h0 = eh.h0();
    let h1 = eh.h1();

    let f0 = conn.face_handle(h0);
    let f1 = conn.face_handle(h1);

    match (f0, f1) {
        (Some(f0), Some(f1)) => {
            let n0 = calc_face_normal(mesh, f0);
            let n1 = calc_face_normal(mesh, f1);

            // Edge direction
            let edge = calc_edge_vector(mesh, h0);
            let edge_len = edge.norm();
            if edge_len == 0.0 {
                return 0.0;
            }
            let e = edge / edge_len;

            // Signed dihedral angle using atan2
            let dot = n0.dot(&n1);
            let cross = n0.cross(&n1);
            let sin_angle = cross.dot(&e);

            sin_angle.atan2(dot)
        }
        _ => 0.0, // boundary edge
    }
}
