use infmesh::{Connectivity, VertexHandle, FaceHandle};

/// Build the test mesh used in most circulator tests:
///     0 ==== 2
///     |\  0 /|
///     | \  / |
///     |2  1 3|
///     | /  \ |
///     |/  1 \|
///     3 ==== 4
fn build_circulator_mesh(conn: &mut Connectivity) -> (Vec<VertexHandle>, Vec<FaceHandle>) {
    let v: Vec<_> = (0..5).map(|_| conn.new_vertex()).collect();
    let mut faces = Vec::new();

    // Face 0: 0-1-2
    faces.push(conn.add_face(&[v[0], v[1], v[2]]).unwrap());
    // Face 1: 1-3-4
    faces.push(conn.add_face(&[v[1], v[3], v[4]]).unwrap());
    // Face 2: 0-3-1
    faces.push(conn.add_face(&[v[0], v[3], v[1]]).unwrap());
    // Face 3: 2-1-4
    faces.push(conn.add_face(&[v[2], v[1], v[4]]).unwrap());

    (v, faces)
}

// ============================================================================
// Vertex-Vertex circulator tests
// Port of: unittests_trimesh_circulator_vertex_vertex.cc
// ============================================================================

/// Port of: VertexVertexIncrement
/// Iterate around interior vertex 1. CW (default C++ direction) should give: 4, 3, 0, 2
#[test]
fn vertex_vertex_cw_interior() {
    let mut conn = Connectivity::new();
    let (v, _) = build_circulator_mesh(&mut conn);

    let neighbors: Vec<usize> = conn.vv_cw_iter(v[1]).map(|v| v.idx()).collect();
    assert_eq!(neighbors, vec![4, 3, 0, 2], "VV CW around vertex 1");
}

/// Port of: VertexVertexBoundaryIncrement
/// Iterate around boundary vertex 2 CW: should give 4, 1, 0
#[test]
fn vertex_vertex_cw_boundary() {
    let mut conn = Connectivity::new();
    let (v, _) = build_circulator_mesh(&mut conn);

    let neighbors: Vec<usize> = conn.vv_cw_iter(v[2]).map(|v| v.idx()).collect();
    assert_eq!(neighbors, vec![4, 1, 0], "VV CW around boundary vertex 2");
}

/// Vertex-vertex CCW should be reverse of CW
#[test]
fn vertex_vertex_ccw_interior() {
    let mut conn = Connectivity::new();
    let (v, _) = build_circulator_mesh(&mut conn);

    let ccw: Vec<usize> = conn.vv_ccw_iter(v[1]).map(|v| v.idx()).collect();

    // CCW should be reverse order of CW (same elements, rotated differently)
    // CW: 4, 3, 0, 2 -> CCW starts from same halfedge but rotates opposite direction
    // CCW starts from same halfedge but walks opposite direction
    // CW: 4, 3, 0, 2 (via cw_rotated = next(opposite))
    // CCW: 4, 2, 0, 3 (via ccw_rotated = opposite(prev))
    assert_eq!(ccw, vec![4, 2, 0, 3], "VV CCW around vertex 1");
}

// ============================================================================
// Vertex-Face circulator tests
// Port of: unittests_trimesh_circulator_vertex_face.cc
// ============================================================================

/// Iterate faces around interior vertex 1
#[test]
fn vertex_face_cw_interior() {
    let mut conn = Connectivity::new();
    let (v, _) = build_circulator_mesh(&mut conn);

    let face_indices: Vec<usize> = conn.vf_cw_iter(v[1]).map(|f| f.idx()).collect();
    // Around vertex 1 CW: faces 3, 1, 2, 0
    assert_eq!(face_indices, vec![3, 1, 2, 0], "VF CW around vertex 1");
}

/// Iterate faces around boundary vertex 0
#[test]
fn vertex_face_cw_boundary() {
    let mut conn = Connectivity::new();
    let (v, _) = build_circulator_mesh(&mut conn);

    let face_indices: Vec<usize> = conn.vf_cw_iter(v[0]).map(|f| f.idx()).collect();
    // Around vertex 0 CW: should get faces 0, 2
    assert_eq!(face_indices, vec![0, 2], "VF CW around boundary vertex 0");
}

// ============================================================================
// Vertex-Edge circulator tests
// ============================================================================

/// Edge count around interior vertex should match valence
#[test]
fn vertex_edge_count() {
    let mut conn = Connectivity::new();
    let (v, _) = build_circulator_mesh(&mut conn);

    let edge_count = conn.ve_cw_iter(v[1]).count();
    assert_eq!(edge_count, 4, "VE around vertex 1 should have 4 edges");

    let edge_count_0 = conn.ve_cw_iter(v[0]).count();
    assert_eq!(edge_count_0, 3, "VE around vertex 0 should have 3 edges");
}

// ============================================================================
// Face-Vertex circulator tests
// Port of: unittests_trimesh_circulator_face_vertex.cc
// ============================================================================

/// Iterate vertices of face 0 (triangle 0-1-2) CCW
#[test]
fn face_vertex_ccw() {
    let mut conn = Connectivity::new();
    let (_, faces) = build_circulator_mesh(&mut conn);

    let verts: Vec<usize> = conn.fv_ccw_iter(faces[0]).map(|v| v.idx()).collect();
    // Face 0 was added as [0, 1, 2], CCW iteration should match
    assert_eq!(verts.len(), 3, "Triangle should have 3 vertices");
    // The starting halfedge may vary, but should contain exactly {0, 1, 2}
    assert!(verts.contains(&0) && verts.contains(&1) && verts.contains(&2),
            "Face 0 vertices should be {{0, 1, 2}}, got {:?}", verts);
}

/// Face-vertex CW should be reverse of CCW
#[test]
fn face_vertex_cw_vs_ccw() {
    let mut conn = Connectivity::new();
    let (_, faces) = build_circulator_mesh(&mut conn);

    let ccw: Vec<usize> = conn.fv_ccw_iter(faces[0]).map(|v| v.idx()).collect();
    let cw: Vec<usize> = conn.fv_cw_iter(faces[0]).map(|v| v.idx()).collect();

    // CW should be reverse of CCW (same elements, different order)
    assert_eq!(ccw.len(), cw.len());
    // CW and CCW of a triangle starting from the same halfedge:
    // If CCW is [a, b, c], CW is [a, c, b] (both start from same halfedge)
    assert_eq!(ccw[0], cw[0], "Both start from same vertex");
}

// ============================================================================
// Face-Face circulator tests
// ============================================================================

/// Adjacent faces of face 3 (2-1-4)
#[test]
fn face_face_ccw() {
    let mut conn = Connectivity::new();
    let (_, faces) = build_circulator_mesh(&mut conn);

    let adj: Vec<usize> = conn.ff_ccw_iter(faces[3]).map(|f| f.idx()).collect();
    // Face 3 (2-1-4) is adjacent to faces with shared edges
    // It shares edges with face 0 (edge 1-2) and face 1 (edge 1-4)
    // Edge 2-4 might be boundary
    assert!(adj.contains(&0), "Face 3 should be adjacent to face 0");
    assert!(adj.contains(&1), "Face 3 should be adjacent to face 1");
}

// ============================================================================
// Face-Edge circulator tests
// ============================================================================

/// Edge count of a triangle face should be 3
#[test]
fn face_edge_count() {
    let mut conn = Connectivity::new();
    let (_, faces) = build_circulator_mesh(&mut conn);

    for fh in &faces {
        let count = conn.fe_ccw_iter(*fh).count();
        assert_eq!(count, 3, "Triangle face {} should have 3 edges", fh.idx());
    }
}

// ============================================================================
// Face-Halfedge circulator tests
// ============================================================================

/// Halfedge loop of a face should form a cycle
#[test]
fn face_halfedge_loop() {
    let mut conn = Connectivity::new();
    let (_, faces) = build_circulator_mesh(&mut conn);

    let hes: Vec<_> = conn.fh_ccw_iter(faces[0]).collect();
    assert_eq!(hes.len(), 3, "Triangle should have 3 halfedges");

    // Each halfedge's next should be the following one in the list
    for i in 0..3 {
        let next_i = (i + 1) % 3;
        assert_eq!(
            conn.next_halfedge_handle(hes[i]),
            hes[next_i],
            "Halfedge loop connectivity broken"
        );
    }
}

// ============================================================================
// Halfedge-Loop circulator tests
// ============================================================================

/// HL iterator should produce same result as FH iterator
#[test]
fn halfedge_loop_matches_face_halfedge() {
    let mut conn = Connectivity::new();
    let (_, faces) = build_circulator_mesh(&mut conn);

    let fh_hes: Vec<_> = conn.fh_ccw_iter(faces[0]).collect();
    let start_he = conn.face_halfedge_handle(faces[0]);
    let hl_hes: Vec<_> = conn.hl_ccw_iter(start_he).collect();

    assert_eq!(fh_hes, hl_hes, "HL and FH iterators should produce same result");
}

// ============================================================================
// Vertex-OutgoingHalfedge circulator
// ============================================================================

/// VOH iterator should yield outgoing halfedges
#[test]
fn vertex_outgoing_halfedge() {
    let mut conn = Connectivity::new();
    let (v, _) = build_circulator_mesh(&mut conn);

    let ohe: Vec<_> = conn.voh_cw_iter(v[1]).collect();
    assert_eq!(ohe.len(), 4, "Vertex 1 should have 4 outgoing halfedges");

    // Each outgoing halfedge should originate from vertex 1
    for hh in &ohe {
        assert_eq!(
            conn.from_vertex_handle(*hh),
            v[1],
            "Outgoing halfedge should come from vertex 1"
        );
    }
}

// ============================================================================
// Vertex-IncomingHalfedge circulator
// ============================================================================

/// VIH iterator should yield incoming halfedges
#[test]
fn vertex_incoming_halfedge() {
    let mut conn = Connectivity::new();
    let (v, _) = build_circulator_mesh(&mut conn);

    let ihe: Vec<_> = conn.vih_cw_iter(v[1]).collect();
    assert_eq!(ihe.len(), 4, "Vertex 1 should have 4 incoming halfedges");

    // Each incoming halfedge should point to vertex 1
    for hh in &ihe {
        assert_eq!(
            conn.to_vertex_handle(*hh),
            v[1],
            "Incoming halfedge should point to vertex 1"
        );
    }
}
