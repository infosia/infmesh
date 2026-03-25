use approx::assert_relative_eq;
use nalgebra::{Point3, Vector3};
use infmesh::{TriMesh, PolyMesh};
use infmesh::geometry::*;

// ============================================================================
// Face Normal tests
// Port of: unittests_normal_calculations.cc
// ============================================================================

/// Single triangle face normal
#[test]
fn face_normal_single_triangle() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));

    let fh = mesh.add_face(&[v0, v1, v2]).unwrap();

    let normal = calc_face_normal(&mesh, fh);

    // Triangle in XY plane, normal should be +Z
    assert_relative_eq!(normal.z, 1.0, epsilon = 1e-5);
    assert_relative_eq!(normal.x, 0.0, epsilon = 1e-5);
    assert_relative_eq!(normal.y, 0.0, epsilon = 1e-5);
}

/// Single quad face normal (polymesh)
#[test]
fn face_normal_single_quad() {
    let mut mesh = PolyMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(1.0, 1.0, 0.0));
    let v3 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));

    let fh = mesh.add_face(&[v0, v1, v2, v3]).unwrap();

    let normal = calc_face_normal(&mesh, fh);

    // Quad in XY plane, normal should be +Z
    assert_relative_eq!(normal.z, 1.0, epsilon = 1e-5);
}

/// Vertex normal on a tetrahedron
#[test]
fn vertex_normal_tetrahedron() {
    let mut mesh = TriMesh::new();

    // Regular-ish tetrahedron
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.5, 1.0, 0.0));
    let v3 = mesh.add_vertex(Point3::new(0.5, 0.5, 1.0));

    mesh.add_face(&[v0, v1, v2]).unwrap();
    mesh.add_face(&[v0, v2, v3]).unwrap();
    mesh.add_face(&[v0, v3, v1]).unwrap();
    mesh.add_face(&[v1, v3, v2]).unwrap();

    // Vertex normals should be unit vectors pointing outward
    for vh in mesh.vertices() {
        let n = calc_vertex_normal(&mesh, vh);
        let len = n.norm();
        assert_relative_eq!(len, 1.0, epsilon = 1e-5);
    }
}

// ============================================================================
// Centroid tests
// Port of: unittests_centroid_calculations.cc
// ============================================================================

/// Face centroid
#[test]
fn face_centroid_triangle() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(3.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 3.0, 0.0));

    let fh = mesh.add_face(&[v0, v1, v2]).unwrap();

    let centroid = calc_face_centroid(&mesh, fh);
    assert_relative_eq!(centroid.x, 1.0, epsilon = 1e-5);
    assert_relative_eq!(centroid.y, 1.0, epsilon = 1e-5);
    assert_relative_eq!(centroid.z, 0.0, epsilon = 1e-5);
}

/// Edge midpoint
#[test]
fn edge_midpoint() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(2.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 2.0, 0.0));

    mesh.add_face(&[v0, v1, v2]).unwrap();

    let hh = mesh.find_halfedge(v0, v1).unwrap();
    let mid = calc_edge_midpoint(&mesh, hh.edge());

    assert_relative_eq!(mid.x, 1.0, epsilon = 1e-5);
    assert_relative_eq!(mid.y, 0.0, epsilon = 1e-5);
    assert_relative_eq!(mid.z, 0.0, epsilon = 1e-5);
}

/// Mesh centroid
#[test]
fn mesh_centroid() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(4.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 4.0, 0.0));

    mesh.add_face(&[v0, v1, v2]).unwrap();

    let centroid = calc_mesh_centroid(&mesh);
    // Average of 3 vertices: (4/3, 4/3, 0)
    assert_relative_eq!(centroid.x, 4.0 / 3.0, epsilon = 1e-5);
    assert_relative_eq!(centroid.y, 4.0 / 3.0, epsilon = 1e-5);
    assert_relative_eq!(centroid.z, 0.0, epsilon = 1e-5);
}

// ============================================================================
// Edge length tests
// ============================================================================

#[test]
fn edge_length_basic() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(3.0, 4.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 4.0, 0.0));

    mesh.add_face(&[v0, v1, v2]).unwrap();

    let hh = mesh.find_halfedge(v0, v1).unwrap();
    let len = calc_edge_length(&mesh, hh.edge());
    assert_relative_eq!(len, 5.0, epsilon = 1e-5);
}

// ============================================================================
// Face area tests
// ============================================================================

#[test]
fn face_area_right_triangle() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(3.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 4.0, 0.0));

    let fh = mesh.add_face(&[v0, v1, v2]).unwrap();

    let area = calc_face_area(&mesh, fh);
    assert_relative_eq!(area, 6.0, epsilon = 1e-5); // 3*4/2
}

#[test]
fn face_area_unit_triangle() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));

    let fh = mesh.add_face(&[v0, v1, v2]).unwrap();

    let area = calc_face_area(&mesh, fh);
    assert_relative_eq!(area, 0.5, epsilon = 1e-5);
}

// ============================================================================
// Dihedral angle tests
// ============================================================================

/// Two coplanar triangles have dihedral angle 0
#[test]
fn dihedral_angle_coplanar() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));
    let v3 = mesh.add_vertex(Point3::new(1.0, 1.0, 0.0));

    mesh.add_face(&[v0, v1, v2]).unwrap();
    mesh.add_face(&[v1, v3, v2]).unwrap();

    let hh = mesh.find_halfedge(v1, v2).unwrap();
    let angle = calc_dihedral_angle(&mesh, hh.edge());
    assert_relative_eq!(angle.abs(), 0.0, epsilon = 1e-5);
}

/// Two perpendicular triangles have dihedral angle pi/2
#[test]
fn dihedral_angle_right_angle() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0)); // in XY
    let v3 = mesh.add_vertex(Point3::new(0.0, 0.0, 1.0)); // in XZ

    mesh.add_face(&[v0, v1, v2]).unwrap();
    mesh.add_face(&[v1, v0, v3]).unwrap();

    let hh = mesh.find_halfedge(v0, v1).unwrap();
    let angle = calc_dihedral_angle(&mesh, hh.edge());
    assert_relative_eq!(angle.abs(), std::f32::consts::FRAC_PI_2, epsilon = 1e-5);
}

/// Boundary edge has dihedral angle 0
#[test]
fn dihedral_angle_boundary() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));

    mesh.add_face(&[v0, v1, v2]).unwrap();

    // All edges are boundary
    for eh in mesh.edges() {
        let angle = calc_dihedral_angle(&mesh, eh);
        assert_relative_eq!(angle, 0.0, epsilon = 1e-5);
    }
}

// ============================================================================
// Edge vector tests
// ============================================================================

#[test]
fn edge_vector_basic() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(1.0, 2.0, 3.0));
    let v1 = mesh.add_vertex(Point3::new(4.0, 5.0, 6.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));

    mesh.add_face(&[v0, v1, v2]).unwrap();

    let hh = mesh.find_halfedge(v0, v1).unwrap();
    let vec = calc_edge_vector(&mesh, hh);
    assert_relative_eq!(vec, Vector3::new(3.0, 3.0, 3.0), epsilon = 1e-5);
}
