use infmesh::{Connectivity, VertexHandle};

/// Helper: create N vertices and return their handles.
fn make_vertices(conn: &mut Connectivity, n: usize) -> Vec<VertexHandle> {
    (0..n).map(|_| conn.new_vertex()).collect()
}

/// Helper: build the triangle cube mesh from the C++ test.
fn build_triangle_cube(conn: &mut Connectivity) -> Vec<VertexHandle> {
    let v = make_vertices(conn, 8);
    // 12 triangle faces forming a cube
    let faces: &[[usize; 3]] = &[
        [0, 1, 3], [1, 2, 3],
        [7, 6, 5], [7, 5, 4],
        [1, 0, 4], [1, 4, 5],
        [2, 1, 5], [2, 5, 6],
        [3, 2, 6], [3, 6, 7],
        [0, 3, 7], [0, 7, 4],
    ];
    for f in faces {
        conn.add_face(&[v[f[0]], v[f[1]], v[f[2]]]).unwrap();
    }
    v
}

/// Helper: build the polygon cube mesh from the C++ test.
fn build_poly_cube(conn: &mut Connectivity) -> Vec<VertexHandle> {
    let v = make_vertices(conn, 8);
    let faces: &[[usize; 4]] = &[
        [0, 1, 2, 3],
        [7, 6, 5, 4],
        [1, 0, 4, 5],
        [2, 1, 5, 6],
        [3, 2, 6, 7],
        [0, 3, 7, 4],
    ];
    for f in faces {
        conn.add_face(&[v[f[0]], v[f[1]], v[f[2]], v[f[3]]]).unwrap();
    }
    v
}

// --- Ported C++ tests from unittests_add_face.cc ---

/// Port of: OpenMeshAddFaceTriangleMesh::AddTrianglesToTrimesh
#[test]
fn add_triangles_to_trimesh() {
    let mut conn = Connectivity::new();
    let v = make_vertices(&mut conn, 4);

    //  1 === 2
    //  |   / |
    //  |  /  |
    //  | /   |
    //  0 === 3
    conn.add_face(&[v[2], v[1], v[0]]).unwrap();
    conn.add_face(&[v[2], v[0], v[3]]).unwrap();

    assert_eq!(conn.n_vertices(), 4, "Wrong number of vertices");
    assert_eq!(conn.n_faces(), 2, "Wrong number of faces");
}

/// Port of: OpenMeshAddFaceTriangleMesh::AddQuadToTrimesh
/// Note: In C++ TriMesh, adding a quad auto-triangulates into 2 triangles.
/// Our Connectivity is a PolyMesh-level structure, so a quad stays as 1 face.
/// TriMesh-specific auto-triangulation will be in Step 8.
/// For now, we test quad addition as a poly mesh.
#[test]
fn add_quad_to_polymesh_stays_quad() {
    let mut conn = Connectivity::new();
    let v = make_vertices(&mut conn, 4);

    conn.add_face(&[v[0], v[1], v[2], v[3]]).unwrap();

    assert_eq!(conn.n_vertices(), 4, "Wrong number of vertices");
    assert_eq!(conn.n_faces(), 1, "Wrong number of faces");
    assert_eq!(conn.n_edges(), 4, "Wrong number of edges");
}

/// Port of: OpenMeshAddFaceTriangleMesh::CreateTriangleMeshCube
#[test]
fn create_triangle_mesh_cube() {
    let mut conn = Connectivity::new();
    build_triangle_cube(&mut conn);

    assert_eq!(conn.n_edges(), 18, "Wrong number of Edges");
    assert_eq!(conn.n_halfedges(), 36, "Wrong number of HalfEdges");
    assert_eq!(conn.n_vertices(), 8, "Wrong number of vertices");
    assert_eq!(conn.n_faces(), 12, "Wrong number of faces");
}

/// Port of: OpenMeshAddFaceTriangleMesh::CreateStrangeConfig
/// Tests that adding a non-manifold face returns an error.
#[test]
fn create_strange_config() {
    let mut conn = Connectivity::new();
    let v = make_vertices(&mut conn, 7);

    conn.add_face(&[v[0], v[1], v[2]]).unwrap();
    conn.add_face(&[v[0], v[3], v[4]]).unwrap();
    conn.add_face(&[v[0], v[5], v[6]]).unwrap();

    // This face should fail -- non-manifold
    let result = conn.add_face(&[v[3], v[0], v[4]]);
    assert!(result.is_err(), "Non-manifold face should fail");

    assert_eq!(conn.n_vertices(), 7, "Wrong number of vertices");
    assert_eq!(conn.n_faces(), 3, "Wrong number of faces");
}

/// Port of: OpenMeshAddFacePolyMesh::AddQuadToPolymesh
#[test]
fn add_quad_to_polymesh() {
    let mut conn = Connectivity::new();
    let v = make_vertices(&mut conn, 4);

    conn.add_face(&[v[0], v[1], v[2], v[3]]).unwrap();

    assert_eq!(conn.n_vertices(), 4, "Wrong number of vertices");
    assert_eq!(conn.n_faces(), 1, "Wrong number of faces");
}

/// Port of: OpenMeshAddFacePolyMesh::CreatePolyMeshCube
#[test]
fn create_poly_mesh_cube() {
    let mut conn = Connectivity::new();
    build_poly_cube(&mut conn);

    assert_eq!(conn.n_edges(), 12, "Wrong number of Edges");
    assert_eq!(conn.n_halfedges(), 24, "Wrong number of HalfEdges");
    assert_eq!(conn.n_vertices(), 8, "Wrong number of vertices");
    assert_eq!(conn.n_faces(), 6, "Wrong number of faces");
}
