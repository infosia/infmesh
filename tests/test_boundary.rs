use infmesh::{Connectivity, FaceHandle, VertexHandle};

/// Build the boundary test mesh from unittests_boundary.cc:
///     0 ==== 2
///     |\  0 /|\
///     | \  / | \
///     |2  1 3|4 5
///     | /  \ | /
///     |/  1 \|/
///     3 ==== 4
///
///     Vertex 6 single (isolated)
fn build_boundary_mesh(conn: &mut Connectivity) -> (Vec<VertexHandle>, Vec<FaceHandle>) {
    let mut v = Vec::new();
    for _ in 0..7 {
        v.push(conn.new_vertex());
    }

    let mut faces = Vec::new();
    // Face 0: 0-1-2
    faces.push(conn.add_face(&[v[0], v[1], v[2]]).unwrap());
    // Face 1: 1-3-4
    faces.push(conn.add_face(&[v[1], v[3], v[4]]).unwrap());
    // Face 2: 0-3-1
    faces.push(conn.add_face(&[v[0], v[3], v[1]]).unwrap());
    // Face 3: 2-1-4
    faces.push(conn.add_face(&[v[2], v[1], v[4]]).unwrap());
    // Face 4: 5-2-4
    faces.push(conn.add_face(&[v[5], v[2], v[4]]).unwrap());

    (v, faces)
}

/// Port of: OpenMeshBoundaryTriangleMesh::TestBoundaryVertex
#[test]
fn test_boundary_vertex() {
    let mut conn = Connectivity::new();
    let (v, _) = build_boundary_mesh(&mut conn);

    assert!(conn.is_boundary_vertex(v[0]),  "Vertex 0 is not boundary!");
    assert!(!conn.is_boundary_vertex(v[1]), "Vertex 1 is boundary!");
    assert!(conn.is_boundary_vertex(v[2]),  "Vertex 2 is not boundary!");
    assert!(conn.is_boundary_vertex(v[3]),  "Vertex 3 is not boundary!");
    assert!(conn.is_boundary_vertex(v[4]),  "Vertex 4 is not boundary!");
    assert!(conn.is_boundary_vertex(v[5]),  "Vertex 5 is not boundary!");

    // Singular (isolated) vertex is boundary
    assert!(conn.is_boundary_vertex(v[6]),  "Singular Vertex 6 is not boundary!");
}

/// Port of: OpenMeshBoundaryTriangleMesh::TestBoundaryFace
#[test]
fn test_boundary_face() {
    let mut conn = Connectivity::new();
    let (_, faces) = build_boundary_mesh(&mut conn);

    assert!(conn.is_boundary_face(faces[0]),  "Face 0 is not boundary!");
    assert!(conn.is_boundary_face(faces[1]),  "Face 1 is not boundary!");
    assert!(conn.is_boundary_face(faces[2]),  "Face 2 is not boundary!");
    assert!(!conn.is_boundary_face(faces[3]), "Face 3 is boundary!");
    assert!(conn.is_boundary_face(faces[4]),  "Face 4 is not boundary!");
}
