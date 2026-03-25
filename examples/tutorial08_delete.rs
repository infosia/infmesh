//! Tutorial 08 — Deleting mesh elements.
//!
//! Port of OpenMesh Tutorial 08: creates a cube, deletes all but one face,
//! runs garbage collection, and writes the result.

use infmesh::io::obj::write_polymesh_obj;
use infmesh::polymesh::PolyMesh;
use nalgebra::Point3;

fn main() {
    let mut mesh = PolyMesh::new();

    // Build the same cube as Tutorial 01.
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

    let face_handles: Vec<_> = [
        [0, 1, 2, 3],
        [7, 6, 5, 4],
        [1, 0, 4, 5],
        [2, 1, 5, 6],
        [3, 2, 6, 7],
        [0, 3, 7, 4],
    ]
    .iter()
    .map(|f| mesh.add_face(&[vhandle[f[0]], vhandle[f[1]], vhandle[f[2]], vhandle[f[3]]]).unwrap())
    .collect();

    println!(
        "Before deletion: {} vertices, {} edges, {} faces",
        mesh.n_vertices(),
        mesh.n_edges(),
        mesh.n_faces(),
    );

    // Delete all faces except the first one.
    for &fh in &face_handles[1..] {
        mesh.delete_face(fh, false);
    }

    // Delete vertices that are now isolated (the four back vertices: 4-7).
    for &vh in &vhandle[4..8] {
        if mesh.is_isolated(vh) {
            mesh.delete_vertex(vh, false);
        }
    }

    println!(
        "After marking:   {} vertices, {} edges, {} faces (some marked deleted)",
        mesh.n_vertices(),
        mesh.n_edges(),
        mesh.n_faces(),
    );

    // Compact the mesh.
    mesh.garbage_collection();

    println!(
        "After GC:        {} vertices, {} edges, {} faces",
        mesh.n_vertices(),
        mesh.n_edges(),
        mesh.n_faces(),
    );

    let path = "output_remaining.obj";
    write_polymesh_obj(path, &mesh, None).expect("Failed to write OBJ");
    println!("Wrote remaining face → {}", path);
}
