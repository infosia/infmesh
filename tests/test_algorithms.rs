use nalgebra::Point3;
use infmesh::Scalar;
use infmesh::TriMesh;
use infmesh::algo::loop_subdivision;
use infmesh::algo::smoother;
use infmesh::algo::decimater;

/// Build a 3x3 grid of triangles (9 vertices, 8 faces).
/// Used as the standard test mesh for subdivision (matches C++ tests).
///
///  6---7---8
///  |\ |\ |
///  | \| \|
///  3---4---5
///  |\ |\ |
///  | \| \|
///  0---1---2
fn make_grid_3x3() -> TriMesh {
    let mut mesh = TriMesh::new();
    let mut verts = Vec::new();

    for j in 0..3 {
        for i in 0..3 {
            verts.push(mesh.add_vertex(Point3::new(i as Scalar, j as Scalar, 0.0)));
        }
    }

    // Lower-left triangles and upper-right triangles for each quad
    for j in 0..2 {
        for i in 0..2 {
            let v00 = verts[j * 3 + i];
            let v10 = verts[j * 3 + i + 1];
            let v01 = verts[(j + 1) * 3 + i];
            let v11 = verts[(j + 1) * 3 + i + 1];
            mesh.add_face(&[v00, v10, v11]).unwrap();
            mesh.add_face(&[v00, v11, v01]).unwrap();
        }
    }

    mesh
}

/// Build a closed tetrahedron (4 vertices, 4 faces).
fn make_tetrahedron() -> TriMesh {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.5, 1.0, 0.0));
    let v3 = mesh.add_vertex(Point3::new(0.5, 0.5, 1.0));

    mesh.add_face(&[v0, v1, v2]).unwrap();
    mesh.add_face(&[v0, v2, v3]).unwrap();
    mesh.add_face(&[v0, v3, v1]).unwrap();
    mesh.add_face(&[v1, v3, v2]).unwrap();

    mesh
}

/// Build an icosahedron (12 vertices, 20 faces) - good test mesh for algorithms.
fn make_icosahedron() -> TriMesh {
    let mut mesh = TriMesh::new();
    let phi = (1.0 + (5.0 as Scalar).sqrt()) / 2.0;

    let v: Vec<_> = [
        (-1.0, phi, 0.0), (1.0, phi, 0.0), (-1.0, -phi, 0.0), (1.0, -phi, 0.0),
        (0.0, -1.0, phi), (0.0, 1.0, phi), (0.0, -1.0, -phi), (0.0, 1.0, -phi),
        (phi, 0.0, -1.0), (phi, 0.0, 1.0), (-phi, 0.0, -1.0), (-phi, 0.0, 1.0),
    ]
    .iter()
    .map(|&(x, y, z)| mesh.add_vertex(Point3::new(x, y, z)))
    .collect();

    let faces = [
        [0,11,5], [0,5,1], [0,1,7], [0,7,10], [0,10,11],
        [1,5,9], [5,11,4], [11,10,2], [10,7,6], [7,1,8],
        [3,9,4], [3,4,2], [3,2,6], [3,6,8], [3,8,9],
        [4,9,5], [2,4,11], [6,2,10], [8,6,7], [9,8,1],
    ];

    for f in &faces {
        mesh.add_face(&[v[f[0]], v[f[1]], v[f[2]]]).unwrap();
    }

    mesh
}

// ============================================================================
// Loop Subdivision tests
// Port of: unittests_subdivider_uniform.cc (Subdivider_Loop)
// ============================================================================

/// One iteration of Loop subdivision on a 3x3 grid.
#[test]
fn loop_subdivision_one_step() {
    let mut mesh = make_grid_3x3();
    assert_eq!(mesh.n_vertices(), 9);
    assert_eq!(mesh.n_faces(), 8);

    loop_subdivision::loop_subdivide(&mut mesh);

    // After 1 step: each edge gets a new vertex, each face splits into 4
    // All faces should still be triangles
    for fh in mesh.faces() {
        assert_eq!(mesh.valence_face(fh), 3, "Face {} not a triangle after subdivision", fh.idx());
    }
    // Should have more vertices and faces
    assert!(mesh.n_vertices() > 9, "Should have more vertices");
    assert!(mesh.n_faces() > 8, "Should have more faces");
}

/// Port of: Subdivider_Loop (3 steps on 3x3 grid)
/// C++ expects: 289 vertices, 512 faces after 3 steps.
#[test]
fn loop_subdivision_three_steps() {
    let mut mesh = make_grid_3x3();
    assert_eq!(mesh.n_vertices(), 9);
    assert_eq!(mesh.n_faces(), 8);

    loop_subdivision::loop_subdivide_n(&mut mesh, 3);

    // C++ test expects 289 vertices, 512 faces
    assert_eq!(mesh.n_vertices(), 289, "Wrong vertex count after 3 Loop subdivisions");
    assert_eq!(mesh.n_faces(), 512, "Wrong face count after 3 Loop subdivisions");
}

/// Loop subdivision preserves triangle property.
#[test]
fn loop_subdivision_all_triangles() {
    let mut mesh = make_tetrahedron();
    loop_subdivision::loop_subdivide_n(&mut mesh, 2);

    for fh in mesh.faces() {
        assert_eq!(mesh.valence_face(fh), 3, "All faces should be triangles");
    }
}

/// Loop subdivision on a closed mesh (tetrahedron).
#[test]
fn loop_subdivision_closed_mesh() {
    let mut mesh = make_tetrahedron();
    assert_eq!(mesh.n_vertices(), 4);
    assert_eq!(mesh.n_faces(), 4);

    loop_subdivision::loop_subdivide(&mut mesh);

    // Closed mesh: 4 faces * 4 = 16 faces, 4 + 6 edges = 10 vertices
    assert_eq!(mesh.n_faces(), 16);
    assert_eq!(mesh.n_vertices(), 10);

    // No boundary vertices in a closed mesh
    for vh in mesh.vertices() {
        assert!(!mesh.is_boundary_vertex(vh), "Closed mesh should have no boundary vertices");
    }
}

// ============================================================================
// Smoother tests
// Port of: unittests_smoother.cc
// ============================================================================

/// Laplacian smoothing preserves topology.
#[test]
fn laplacian_smooth_preserves_topology() {
    let mut mesh = make_icosahedron();
    let orig_v = mesh.n_vertices();
    let orig_f = mesh.n_faces();
    let orig_e = mesh.n_edges();

    let opts = smoother::SmoothOptions {
        iterations: 5,
        lambda: 0.5,
        fix_boundary: true,
    };

    smoother::laplacian_smooth(&mut mesh, &opts);

    assert_eq!(mesh.n_vertices(), orig_v, "Smoothing should not change vertex count");
    assert_eq!(mesh.n_faces(), orig_f, "Smoothing should not change face count");
    assert_eq!(mesh.n_edges(), orig_e, "Smoothing should not change edge count");
}

/// Laplacian smoothing moves vertices toward neighbors.
#[test]
fn laplacian_smooth_reduces_noise() {
    let mut mesh = TriMesh::new();
    // Create a flat grid with one noisy vertex
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.5, 1.0, 0.0));
    let v3 = mesh.add_vertex(Point3::new(0.5, 0.5, 5.0)); // noisy vertex (way above the plane)

    mesh.add_face(&[v0, v1, v3]).unwrap();
    mesh.add_face(&[v1, v2, v3]).unwrap();
    mesh.add_face(&[v2, v0, v3]).unwrap();

    let z_before = mesh.point(v3).z;

    let opts = smoother::SmoothOptions {
        iterations: 10,
        lambda: 0.5,
        fix_boundary: true,
    };
    smoother::laplacian_smooth(&mut mesh, &opts);

    let z_after = mesh.point(v3).z;
    assert!(z_after < z_before, "Noisy vertex should move closer to neighbors");
}

/// Taubin smoothing (no-shrink variant).
#[test]
fn taubin_smooth_basic() {
    let mut mesh = make_icosahedron();
    let orig_v = mesh.n_vertices();

    smoother::taubin_smooth(&mut mesh, 5, 0.5, 0.53, false);

    assert_eq!(mesh.n_vertices(), orig_v, "Taubin should preserve vertex count");
}

// ============================================================================
// Decimater tests
// Port of: unittests_decimater.cc
// ============================================================================

/// Decimate a subdivided mesh.
#[test]
fn decimate_basic() {
    // Create a mesh with enough vertices to decimate
    let mut mesh = make_icosahedron();
    // Subdivide to get more vertices
    loop_subdivision::loop_subdivide(&mut mesh);

    let orig_v = mesh.n_vertices();
    let orig_f = mesh.n_faces();
    assert!(orig_v > 40, "Need enough vertices to decimate, got {}", orig_v);

    let target = orig_v / 2;
    let collapses = decimater::decimate(&mut mesh, target);

    assert!(collapses > 0, "Should have performed some collapses");
    assert!(mesh.n_vertices() <= target, "Should have at most {} vertices, got {}", target, mesh.n_vertices());
    assert!(mesh.n_faces() < orig_f, "Should have fewer faces");

    // All remaining faces should be triangles
    for fh in mesh.faces() {
        assert_eq!(mesh.valence_face(fh), 3, "Face {} should be a triangle", fh.idx());
    }
}

/// Decimate already at target should do nothing.
#[test]
fn decimate_at_target() {
    let mut mesh = make_tetrahedron();
    let collapses = decimater::decimate(&mut mesh, 100);
    assert_eq!(collapses, 0, "Should not decimate if already below target");
    assert_eq!(mesh.n_vertices(), 4);
    assert_eq!(mesh.n_faces(), 4);
}

/// Decimate a larger mesh (subdivided icosahedron, 2 levels).
#[test]
fn decimate_larger_mesh() {
    let mut mesh = make_icosahedron();
    loop_subdivision::loop_subdivide_n(&mut mesh, 2);

    let orig_v = mesh.n_vertices();
    let orig_f = mesh.n_faces();

    // Decimate to roughly half
    let target = orig_v / 2;
    decimater::decimate(&mut mesh, target);

    assert!(mesh.n_vertices() <= target);
    assert!(mesh.n_faces() < orig_f);
}

/// Decimate to a face count target.
#[test]
fn decimate_to_faces() {
    let mut mesh = make_icosahedron();
    loop_subdivision::loop_subdivide(&mut mesh);

    let orig_f = mesh.n_faces();
    let target_f = orig_f / 2;

    decimater::decimate_to_faces(&mut mesh, target_f);

    // Should have roughly half the faces (approximate due to Euler formula estimation)
    assert!(mesh.n_faces() < orig_f, "Should have fewer faces after decimation");
}
