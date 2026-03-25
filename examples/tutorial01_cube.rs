//! Tutorial 01 — Building a cube from scratch.
//!
//! Port of OpenMesh Tutorial 01: creates an 8-vertex cube with 6 quad faces
//! and writes it to an OBJ file.

use infmesh::io::obj::write_polymesh_obj;
use infmesh::polymesh::PolyMesh;
use nalgebra::Point3;

fn main() {
    let mut mesh = PolyMesh::new();

    // Add the 8 corner vertices of a unit cube.
    let vhandle: Vec<_> = [
        Point3::new(-1.0, -1.0,  1.0),
        Point3::new( 1.0, -1.0,  1.0),
        Point3::new( 1.0,  1.0,  1.0),
        Point3::new(-1.0,  1.0,  1.0),
        Point3::new(-1.0, -1.0, -1.0),
        Point3::new( 1.0, -1.0, -1.0),
        Point3::new( 1.0,  1.0, -1.0),
        Point3::new(-1.0,  1.0, -1.0),
    ]
    .iter()
    .map(|p| mesh.add_vertex(*p))
    .collect();

    // Add the 6 quad faces.
    mesh.add_face(&[vhandle[0], vhandle[1], vhandle[2], vhandle[3]]).unwrap();
    mesh.add_face(&[vhandle[7], vhandle[6], vhandle[5], vhandle[4]]).unwrap();
    mesh.add_face(&[vhandle[1], vhandle[0], vhandle[4], vhandle[5]]).unwrap();
    mesh.add_face(&[vhandle[2], vhandle[1], vhandle[5], vhandle[6]]).unwrap();
    mesh.add_face(&[vhandle[3], vhandle[2], vhandle[6], vhandle[7]]).unwrap();
    mesh.add_face(&[vhandle[0], vhandle[3], vhandle[7], vhandle[4]]).unwrap();

    let path = "output_cube.obj";
    write_polymesh_obj(path, &mesh, None).expect("Failed to write OBJ");
    println!(
        "Wrote cube: {} vertices, {} edges, {} faces → {}",
        mesh.n_vertices(),
        mesh.n_edges(),
        mesh.n_faces(),
        path,
    );
}
