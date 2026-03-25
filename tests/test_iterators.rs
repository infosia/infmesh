use infmesh::{Connectivity, VertexHandle};

fn make_two_triangles(conn: &mut Connectivity) -> Vec<VertexHandle> {
    let v: Vec<_> = (0..4).map(|_| conn.new_vertex()).collect();
    //  1 === 2
    //  |   / |
    //  |  /  |
    //  | /   |
    //  0 === 3
    conn.add_face(&[v[2], v[1], v[0]]).unwrap();
    conn.add_face(&[v[2], v[0], v[3]]).unwrap();
    v
}

/// Port of: OpenMeshIterators::VertexIter
#[test]
fn vertex_iter() {
    let mut conn = Connectivity::new();
    make_two_triangles(&mut conn);

    let indices: Vec<usize> = conn.vertices().map(|v| v.idx()).collect();
    assert_eq!(indices, vec![0, 1, 2, 3]);
}

/// Port of: OpenMeshIterators::VertexIterStartPosition
#[test]
fn vertex_iter_start_position() {
    let mut conn = Connectivity::new();
    make_two_triangles(&mut conn);

    // Skip first 2 vertices by using skip()
    let indices: Vec<usize> = conn.vertices().skip(2).map(|v| v.idx()).collect();
    assert_eq!(indices, vec![2, 3]);
}

/// Port of: OpenMeshIterators::EdgeIter
#[test]
fn edge_iter() {
    let mut conn = Connectivity::new();
    make_two_triangles(&mut conn);

    // 2 triangles sharing an edge = 5 edges
    assert_eq!(conn.n_edges(), 5);
    let indices: Vec<usize> = conn.edges().map(|e| e.idx()).collect();
    assert_eq!(indices, vec![0, 1, 2, 3, 4]);
}

/// Port of: part of OpenMeshIterators::EdgeIter - edge to halfedge checks
#[test]
fn edge_halfedge_relationship() {
    let mut conn = Connectivity::new();
    make_two_triangles(&mut conn);

    for eh in conn.edges() {
        let h0 = eh.h0();
        let h1 = eh.h1();
        // to_vertex of h0 should be from_vertex of h1
        assert_eq!(
            conn.to_vertex_handle(h0),
            conn.from_vertex_handle(h1),
            "Edge {} halfedge vertex mismatch", eh.idx()
        );
        assert_eq!(
            conn.from_vertex_handle(h0),
            conn.to_vertex_handle(h1),
            "Edge {} halfedge vertex mismatch (reverse)", eh.idx()
        );
    }
}

/// Port of: OpenMeshIterators related - face iterator
#[test]
fn face_iter() {
    let mut conn = Connectivity::new();
    make_two_triangles(&mut conn);

    assert_eq!(conn.n_faces(), 2);
    let indices: Vec<usize> = conn.faces().map(|f| f.idx()).collect();
    assert_eq!(indices, vec![0, 1]);
}

/// Port of: OpenMeshIterators related - halfedge iterator
#[test]
fn halfedge_iter() {
    let mut conn = Connectivity::new();
    make_two_triangles(&mut conn);

    assert_eq!(conn.n_halfedges(), 10); // 5 edges * 2
    let indices: Vec<usize> = conn.halfedges().map(|h| h.idx()).collect();
    assert_eq!(indices, (0..10).collect::<Vec<_>>());
}

/// Port of: OpenMeshTrimeshNavigation::EdgeHalfedgeDefault
#[test]
fn edge_halfedge_default() {
    let mut conn = Connectivity::new();
    make_two_triangles(&mut conn);

    for eh in conn.edges() {
        let h0 = conn.edge_halfedge_handle(eh, 0);
        let h1 = conn.edge_halfedge_handle(eh, 1);
        assert_eq!(h0, eh.h0());
        assert_eq!(h1, eh.h1());
        assert_eq!(h0.opposite(), h1);
        assert_eq!(h1.opposite(), h0);
    }
}

/// ExactSizeIterator check
#[test]
fn iter_exact_size() {
    let mut conn = Connectivity::new();
    make_two_triangles(&mut conn);

    assert_eq!(conn.vertices().len(), 4);
    assert_eq!(conn.edges().len(), 5);
    assert_eq!(conn.halfedges().len(), 10);
    assert_eq!(conn.faces().len(), 2);
}

/// DoubleEndedIterator check
#[test]
fn iter_double_ended() {
    let mut conn = Connectivity::new();
    make_two_triangles(&mut conn);

    let indices: Vec<usize> = conn.vertices().rev().map(|v| v.idx()).collect();
    assert_eq!(indices, vec![3, 2, 1, 0]);
}
