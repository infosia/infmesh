use infmesh::io::obj::*;
use infmesh::TriMesh;
use nalgebra::{Point3, Vector3};

const TEST_FILES: &str = "tests/TestFiles";

/// Port of: LoadSimpleOBJ
/// Load cube-minimal.obj and verify topology.
#[test]
fn load_simple_obj() {
    let path = format!("{}/cube-minimal.obj", TEST_FILES);
    let (mesh, _data) = read_trimesh_obj(&path).expect("Failed to read OBJ");

    assert_eq!(mesh.n_vertices(), 8, "Wrong number of vertices");
    assert_eq!(mesh.n_edges(), 18, "Wrong number of edges");
    assert_eq!(mesh.n_faces(), 12, "Wrong number of faces");
}

/// Verify vertex positions from the cube.
#[test]
fn load_simple_obj_positions() {
    let path = format!("{}/cube-minimal.obj", TEST_FILES);
    let (mesh, _) = read_trimesh_obj(&path).unwrap();

    // First vertex should be (0,0,0)
    let p0 = mesh.point(infmesh::VertexHandle::new(0));
    assert_eq!(*p0, Point3::new(0.0, 0.0, 0.0));

    // Last vertex should be (1,1,1)
    let p7 = mesh.point(infmesh::VertexHandle::new(7));
    assert_eq!(*p7, Point3::new(1.0, 1.0, 1.0));
}

/// Port of: ReadWriteReadSimpleOBJ
/// Round-trip test: read, write, read back.
#[test]
fn read_write_read_obj() {
    let path = format!("{}/cube-minimal.obj", TEST_FILES);
    let (mesh, _) = read_trimesh_obj(&path).unwrap();

    // Write to temp file
    let out_path = "target/test_rw_cube.obj";
    write_trimesh_obj(out_path, &mesh, None).expect("Failed to write OBJ");

    // Read back
    let (mesh2, _) = read_trimesh_obj(out_path).expect("Failed to read back OBJ");

    assert_eq!(mesh2.n_vertices(), mesh.n_vertices());
    assert_eq!(mesh2.n_faces(), mesh.n_faces());
    assert_eq!(mesh2.n_edges(), mesh.n_edges());

    // Verify positions match
    for vh in mesh.vertices() {
        let p1 = mesh.point(vh);
        let p2 = mesh2.point(vh);
        assert!((p1 - p2).norm() < 1e-5, "Position mismatch at vertex {}", vh.idx());
    }

    // Cleanup
    let _ = std::fs::remove_file(out_path);
}

/// Write with normals and read back.
#[test]
fn write_with_normals() {
    let mut mesh = TriMesh::new();
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));
    mesh.add_face(&[v0, v1, v2]).unwrap();

    let normals = vec![
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 0.0, 1.0),
    ];

    let out_path = "target/test_normals.obj";
    write_trimesh_obj(out_path, &mesh, Some(&normals)).unwrap();

    let (mesh2, _) = read_trimesh_obj(out_path).unwrap();
    assert_eq!(mesh2.n_vertices(), 3);
    assert_eq!(mesh2.n_faces(), 1);

    let _ = std::fs::remove_file(out_path);
}

/// Read as PolyMesh (preserving polygon faces).
#[test]
fn load_polymesh_obj() {
    let path = format!("{}/cube-minimal.obj", TEST_FILES);
    let (mesh, _) = read_polymesh_obj(&path).unwrap();

    assert_eq!(mesh.n_vertices(), 8);
    assert_eq!(mesh.n_faces(), 12); // Still triangles since the OBJ has triangles
}

/// Write and read a programmatically created mesh.
#[test]
fn write_read_programmatic() {
    let mut mesh = TriMesh::new();

    // Create a simple pyramid
    let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = mesh.add_vertex(Point3::new(0.5, 1.0, 0.0));
    let v3 = mesh.add_vertex(Point3::new(0.5, 0.5, 1.0));

    mesh.add_face(&[v0, v1, v2]).unwrap();
    mesh.add_face(&[v0, v2, v3]).unwrap();
    mesh.add_face(&[v0, v3, v1]).unwrap();
    mesh.add_face(&[v1, v3, v2]).unwrap();

    let out_path = "target/test_pyramid.obj";
    write_trimesh_obj(out_path, &mesh, None).unwrap();

    let (mesh2, _) = read_trimesh_obj(out_path).unwrap();

    assert_eq!(mesh2.n_vertices(), 4);
    assert_eq!(mesh2.n_faces(), 4);
    assert_eq!(mesh2.n_edges(), 6);

    let _ = std::fs::remove_file(out_path);
}
