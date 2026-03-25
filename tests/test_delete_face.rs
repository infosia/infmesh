use infmesh::{Connectivity, VertexHandle, FaceHandle};

/// Build a triangle mesh cube (8 vertices, 12 triangles, 18 edges).
/// Port of the cube setup used in unittests_delete_face.cc
fn make_triangle_cube(conn: &mut Connectivity) -> (Vec<VertexHandle>, Vec<FaceHandle>) {
    let v: Vec<_> = (0..8).map(|_| conn.new_vertex()).collect();
    let mut faces = Vec::new();

    // Front face
    faces.push(conn.add_face(&[v[0], v[1], v[3]]).unwrap());
    faces.push(conn.add_face(&[v[1], v[2], v[3]]).unwrap());

    // Back face
    faces.push(conn.add_face(&[v[7], v[6], v[5]]).unwrap());
    faces.push(conn.add_face(&[v[7], v[5], v[4]]).unwrap());

    // Bottom face
    faces.push(conn.add_face(&[v[1], v[0], v[4]]).unwrap());
    faces.push(conn.add_face(&[v[1], v[4], v[5]]).unwrap());

    // Right face
    faces.push(conn.add_face(&[v[2], v[1], v[5]]).unwrap());
    faces.push(conn.add_face(&[v[2], v[5], v[6]]).unwrap());

    // Top face
    faces.push(conn.add_face(&[v[3], v[2], v[6]]).unwrap());
    faces.push(conn.add_face(&[v[3], v[6], v[7]]).unwrap());

    // Left face
    faces.push(conn.add_face(&[v[0], v[3], v[7]]).unwrap());
    faces.push(conn.add_face(&[v[0], v[7], v[4]]).unwrap());

    (v, faces)
}

/// Build a polygon mesh cube (8 vertices, 6 quad faces, 12 edges).
fn make_poly_cube(conn: &mut Connectivity) -> (Vec<VertexHandle>, Vec<FaceHandle>) {
    let v: Vec<_> = (0..8).map(|_| conn.new_vertex()).collect();
    let mut faces = Vec::new();

    faces.push(conn.add_face(&[v[0], v[1], v[2], v[3]]).unwrap());
    faces.push(conn.add_face(&[v[7], v[6], v[5], v[4]]).unwrap());
    faces.push(conn.add_face(&[v[1], v[0], v[4], v[5]]).unwrap());
    faces.push(conn.add_face(&[v[2], v[1], v[5], v[6]]).unwrap());
    faces.push(conn.add_face(&[v[3], v[2], v[6], v[7]]).unwrap());
    faces.push(conn.add_face(&[v[0], v[3], v[7], v[4]]).unwrap());

    (v, faces)
}

/// Port of: DeleteHalfTriangleMeshCubeWithEdgeStatus
/// (Our Rust version always has status, so this subsumes the NoEdgeStatus variant)
#[test]
fn delete_half_triangle_cube() {
    let mut conn = Connectivity::new();
    let (_, faces) = make_triangle_cube(&mut conn);

    // Check setup
    assert_eq!(conn.n_edges(), 18);
    assert_eq!(conn.n_halfedges(), 36);
    assert_eq!(conn.n_vertices(), 8);
    assert_eq!(conn.n_faces(), 12);

    // Delete first 6 faces
    let n_to_delete = conn.n_faces() / 2;
    assert_eq!(n_to_delete, 6);
    for i in 0..n_to_delete {
        conn.delete_face(faces[i], true);
    }

    // After marking: counts unchanged (lazy deletion)
    assert_eq!(conn.n_edges(), 18, "Edges after marking");
    assert_eq!(conn.n_halfedges(), 36, "Halfedges after marking");
    assert_eq!(conn.n_vertices(), 8, "Vertices after marking");
    assert_eq!(conn.n_faces(), 12, "Faces after marking");

    // Garbage collection compacts
    conn.garbage_collection();

    // C++ "WithEdgeStatus" expects: 13 edges, 8 vertices, 6 faces
    assert_eq!(conn.n_edges(), 13, "Edges after GC");
    assert_eq!(conn.n_vertices(), 8, "Vertices after GC");
    assert_eq!(conn.n_faces(), 6, "Faces after GC");
}

/// Port of: DeleteteHalfPolyMeshCubeWithEdgeStatus
#[test]
fn delete_half_poly_cube() {
    let mut conn = Connectivity::new();
    let (_, faces) = make_poly_cube(&mut conn);

    // Check setup
    assert_eq!(conn.n_edges(), 12);
    assert_eq!(conn.n_halfedges(), 24);
    assert_eq!(conn.n_vertices(), 8);
    assert_eq!(conn.n_faces(), 6);

    // Delete first 3 faces
    let n_to_delete = conn.n_faces() / 2;
    assert_eq!(n_to_delete, 3);
    for i in 0..n_to_delete {
        conn.delete_face(faces[i], true);
    }

    // After marking: counts unchanged
    assert_eq!(conn.n_edges(), 12, "Edges after marking");
    assert_eq!(conn.n_halfedges(), 24, "Halfedges after marking");
    assert_eq!(conn.n_vertices(), 8, "Vertices after marking");
    assert_eq!(conn.n_faces(), 6, "Faces after marking");

    // Garbage collection
    conn.garbage_collection();

    // C++ "WithEdgeStatus" expects: 10 edges, 8 vertices, 3 faces
    assert_eq!(conn.n_edges(), 10, "Edges after GC");
    assert_eq!(conn.n_vertices(), 8, "Vertices after GC");
    assert_eq!(conn.n_faces(), 3, "Faces after GC");
}

/// Test that delete_vertex removes all incident faces
#[test]
fn delete_vertex_removes_faces() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..4).map(|_| conn.new_vertex()).collect();

    // Two triangles sharing edge v[0]-v[2]
    conn.add_face(&[v[0], v[1], v[2]]).unwrap();
    conn.add_face(&[v[0], v[2], v[3]]).unwrap();

    assert_eq!(conn.n_faces(), 2);

    conn.delete_vertex(v[0], true);

    conn.garbage_collection();

    assert_eq!(conn.n_faces(), 0, "All faces incident to v0 should be deleted");
}

/// Test delete_edge removes both adjacent faces
#[test]
fn delete_edge_removes_adjacent_faces() {
    let mut conn = Connectivity::new();
    let v: Vec<_> = (0..4).map(|_| conn.new_vertex()).collect();

    // Two triangles sharing edge v[0]-v[2]
    conn.add_face(&[v[0], v[1], v[2]]).unwrap();
    conn.add_face(&[v[0], v[2], v[3]]).unwrap();

    // Find the shared edge
    let hh = conn.find_halfedge(v[0], v[2]).unwrap();
    let eh = hh.edge();

    conn.delete_edge(eh, false);

    conn.garbage_collection();

    assert_eq!(conn.n_faces(), 0, "Both faces adjacent to edge should be deleted");
}
