use infmesh::{Connectivity, HalfedgeHandle, FaceHandle, TriMesh};
use nalgebra::Point3;

// ============================================================================
// Flip tests
// ============================================================================

#[test]
fn flip_basic() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..4).map(|_| conn.new_vertex()).collect();

    // Two triangles sharing edge v[0]-v[2]
    //  1 --- 2
    //  | \   |
    //  |  \  |
    //  |   \ |
    //  0 --- 3
    conn.add_face(&[v[0], v[1], v[2]]).unwrap();
    conn.add_face(&[v[0], v[2], v[3]]).unwrap();

    // Find the shared edge
    let hh = conn.find_halfedge(v[0], v[2]).unwrap();
    let eh = hh.edge();

    assert!(conn.is_flip_ok(eh));
    conn.flip(eh).unwrap();

    // After flip: edge should connect v[1] to v[3]
    let h0 = eh.h0();
    let h1 = eh.h1();
    let new_v0 = conn.to_vertex_handle(h0);
    let new_v1 = conn.to_vertex_handle(h1);
    assert!(
        (new_v0 == v[1] && new_v1 == v[3]) || (new_v0 == v[3] && new_v1 == v[1]),
        "Flipped edge should connect v1 and v3"
    );

    // Should still have 2 faces and 5 edges
    assert_eq!(conn.n_faces(), 2);
    assert_eq!(conn.n_edges(), 5);
}

#[test]
fn flip_boundary_not_ok() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..3).map(|_| conn.new_vertex()).collect();

    conn.add_face(&[v[0], v[1], v[2]]).unwrap();

    // All edges are boundary, flip should not be ok
    for ei in 0..conn.n_edges() {
        let eh = infmesh::EdgeHandle::new(ei as u32);
        assert!(!conn.is_flip_ok(eh), "Boundary edge {} should not be flippable", ei);
    }
}

// ============================================================================
// Collapse tests - Port of unittests_trimesh_collapse.cc
// ============================================================================

/// Port of: CollapseQuadWithCenter
#[test]
fn collapse_quad_with_center() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..5).map(|_| conn.new_vertex()).collect();

    // 0--------1
    // |\      /|
    // | \    / |
    // |  \  /  |
    // |    2   |
    // |  /  \  |
    // | /    \ |
    // 3--------4
    conn.add_face(&[v[0], v[2], v[1]]).unwrap();
    conn.add_face(&[v[0], v[3], v[2]]).unwrap();
    conn.add_face(&[v[4], v[2], v[3]]).unwrap();
    conn.add_face(&[v[4], v[1], v[2]]).unwrap();

    let v2v1 = conn.find_halfedge(v[2], v[1]).unwrap();
    assert!(conn.is_collapse_ok(v2v1));
    conn.collapse(v2v1);
}

/// Port of: CollapseTetrahedronComplex
#[test]
fn collapse_tetrahedron_complex() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..4).map(|_| conn.new_vertex()).collect();

    // Tetrahedron: 4 faces
    conn.add_face(&[v[0], v[1], v[2]]).unwrap(); // face 0
    conn.add_face(&[v[0], v[2], v[3]]).unwrap(); // face 1
    conn.add_face(&[v[2], v[1], v[3]]).unwrap(); // face 2
    conn.add_face(&[v[3], v[1], v[0]]).unwrap(); // face 3

    let v0v1 = HalfedgeHandle::new(0);
    let v1v0 = v0v1.opposite();

    assert!(conn.is_collapse_ok(v0v1));
    assert!(conn.is_collapse_ok(v1v0));

    // Collapse v0->v1 (v0 is removed, v1 survives)
    conn.collapse(v0v1);

    // Should still have 4 faces (lazy deletion)
    assert_eq!(conn.n_faces(), 4);

    // Face 0 and face 3 should be deleted
    assert!(conn.is_deleted_face(FaceHandle::new(0)));
    assert!(!conn.is_deleted_face(FaceHandle::new(1)));
    assert!(!conn.is_deleted_face(FaceHandle::new(2)));
    assert!(conn.is_deleted_face(FaceHandle::new(3)));

    // After garbage collection
    conn.garbage_collection();

    assert_eq!(conn.n_faces(), 2);
    assert_eq!(conn.n_vertices(), 3);

    // No further collapses should be possible (it's a 2-face flat mesh)
    for i in 0..conn.n_halfedges() {
        let hh = HalfedgeHandle::new(i as u32);
        assert!(!conn.is_collapse_ok(hh), "Collapse should not be ok for halfedge {}", i);
    }
}

/// Port of: CollapseTetrahedron - sequential collapses on a pyramid
#[test]
fn collapse_tetrahedron_sequential() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..5).map(|_| conn.new_vertex()).collect();

    // Pyramid: 5 faces
    conn.add_face(&[v[0], v[4], v[2]]).unwrap();
    conn.add_face(&[v[3], v[4], v[0]]).unwrap();
    conn.add_face(&[v[2], v[4], v[3]]).unwrap();
    conn.add_face(&[v[2], v[1], v[0]]).unwrap();
    conn.add_face(&[v[0], v[1], v[3]]).unwrap();

    // Collapse halfedge 0 (from v0 to v4)
    let heh1 = HalfedgeHandle::new(0);
    assert_eq!(conn.to_vertex_handle(heh1).idx(), 4);
    assert_eq!(conn.from_vertex_handle(heh1).idx(), 0);

    assert!(conn.is_collapse_ok(heh1));
    conn.collapse(heh1);

    // Second collapse: halfedge 2 (from v4 to v2)
    let heh2 = HalfedgeHandle::new(2);
    assert_eq!(conn.to_vertex_handle(heh2).idx(), 2);
    assert_eq!(conn.from_vertex_handle(heh2).idx(), 4);

    assert!(conn.is_collapse_ok(heh2));
    conn.collapse(heh2);

    // Third collapse should fail
    let heh3 = HalfedgeHandle::new(6);
    assert!(!conn.is_collapse_ok(heh3));
}

/// Port of: LargeCollapseHalfEdge
#[test]
fn large_collapse_halfedge() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..7).map(|_| conn.new_vertex()).collect();

    //  0 ---- 1 ---- 2
    //  |    / |    / |
    //  |   /  |   /  |
    //  |  /   |  /   |
    //  | /    | /    |
    //  3 ---- 4 ---- 5
    //         |
    //         6

    conn.add_face(&[v[0], v[3], v[1]]).unwrap();
    conn.add_face(&[v[1], v[3], v[4]]).unwrap();
    conn.add_face(&[v[1], v[4], v[2]]).unwrap();
    conn.add_face(&[v[2], v[4], v[5]]).unwrap();
    conn.add_face(&[v[4], v[3], v[6]]).unwrap();
    conn.add_face(&[v[4], v[6], v[5]]).unwrap();

    // Collapse v3->v4 (v3 is removed)
    let heh = conn.find_halfedge(v[3], v[4]).unwrap();
    assert!(conn.is_collapse_ok(heh));
    conn.collapse(heh);

    // v3 should be deleted, v4 survives
    assert!(conn.is_deleted_vertex(v[3]));
    assert!(!conn.is_deleted_vertex(v[4]));
}

// ============================================================================
// Split tests
// ============================================================================

/// Split an interior edge in a triangle mesh (2-to-4 split)
#[test]
fn split_edge_interior() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..4).map(|_| conn.new_vertex()).collect();

    // Two triangles sharing edge v[0]-v[2]
    conn.add_face(&[v[0], v[1], v[2]]).unwrap();
    conn.add_face(&[v[0], v[2], v[3]]).unwrap();

    assert_eq!(conn.n_faces(), 2);
    assert_eq!(conn.n_edges(), 5);

    // Split the shared edge
    let hh = conn.find_halfedge(v[0], v[2]).unwrap();
    let eh = hh.edge();
    let new_v = conn.new_vertex();
    conn.split_edge(eh, new_v);

    // Interior edge split: 2 faces -> 4 faces
    assert_eq!(conn.n_faces(), 4);
    // New vertex should have valence 4
    assert_eq!(conn.valence_vertex(new_v), 4);
    // All faces should be triangles
    for fh in conn.faces() {
        assert_eq!(conn.valence_face(fh), 3, "Face {} should be a triangle", fh.idx());
    }
}

/// Split a boundary edge (1-to-2 split)
#[test]
fn split_edge_boundary() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..3).map(|_| conn.new_vertex()).collect();

    conn.add_face(&[v[0], v[1], v[2]]).unwrap();

    assert_eq!(conn.n_faces(), 1);

    // Find a boundary edge
    let hh = conn.find_halfedge(v[0], v[1]).unwrap();
    let eh = hh.edge();
    // This edge is boundary on one side (the opposite side of the face)

    let new_v = conn.new_vertex();
    conn.split_edge(eh, new_v);

    // Boundary edge split: 1 face -> 2 faces
    assert_eq!(conn.n_faces(), 2);
    // All faces should be triangles
    for fh in conn.faces() {
        assert_eq!(conn.valence_face(fh), 3, "Face {} should be a triangle", fh.idx());
    }
}

/// Split face (1-to-3)
#[test]
fn split_face_basic() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..3).map(|_| conn.new_vertex()).collect();

    let fh = conn.add_face(&[v[0], v[1], v[2]]).unwrap();
    assert_eq!(conn.n_faces(), 1);

    let center = conn.new_vertex();
    conn.split_face(fh, center);

    // 1-to-3 split
    assert_eq!(conn.n_faces(), 3);
    assert_eq!(conn.valence_vertex(center), 3);
    // All faces should be triangles
    for fh in conn.faces() {
        assert_eq!(conn.valence_face(fh), 3, "Face {} should be a triangle", fh.idx());
    }
}

// ============================================================================
// TriMesh geometry tests
// ============================================================================

#[test]
fn trimesh_add_vertex_and_face() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));

    let fh = mesh.add_face(&[v0, v1, v2]).unwrap();

    assert_eq!(mesh.n_vertices(), 3);
    assert_eq!(mesh.n_faces(), 1);
    assert_eq!(mesh.valence_face(fh), 3);

    assert_eq!(*mesh.point(v0), Point3::new(0.0, 0.0, 0.0));
    assert_eq!(*mesh.point(v1), Point3::new(1.0, 0.0, 0.0));
}

#[test]
fn trimesh_auto_triangulate() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(1.0, 1.0, 0.0));
    let v3 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));

    // Adding a quad to a TriMesh auto-triangulates it
    mesh.add_face(&[v0, v1, v2, v3]).unwrap();

    // Should produce 2 triangles from the quad
    assert_eq!(mesh.n_faces(), 2);
    for fh in mesh.faces() {
        assert_eq!(mesh.valence_face(fh), 3);
    }
}

#[test]
fn trimesh_deref_access() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));

    mesh.add_face(&[v0, v1, v2]).unwrap();

    // Test that Deref to Connectivity works
    assert_eq!(mesh.n_vertices(), 3);
    assert!(mesh.is_boundary_vertex(v0));

    let neighbors: Vec<_> = mesh.vv_cw_iter(v0).collect();
    assert_eq!(neighbors.len(), 2);
}
