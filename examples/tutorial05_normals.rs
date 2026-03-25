//! Tutorial 05 — Working with vertex normals.
//!
//! Port of OpenMesh Tutorial 05: loads a mesh, computes vertex normals from
//! face normals, then moves every vertex along its normal.

use std::env;

use infmesh::geometry::{calc_face_normal, calc_vertex_normal};
use infmesh::io::obj::{read_trimesh_obj, write_trimesh_obj};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input.obj> <output.obj>", args[0]);
        std::process::exit(1);
    }

    let input = &args[1];
    let output = &args[2];

    let (mut mesh, _) = read_trimesh_obj(input).expect("Failed to read input mesh");
    println!(
        "Loaded: {} vertices, {} faces",
        mesh.n_vertices(),
        mesh.n_faces(),
    );

    // Print some face normals.
    for fh in mesh.faces().take(5) {
        let n = calc_face_normal(&mesh, fh);
        println!("Face {} normal: ({:.4}, {:.4}, {:.4})", fh.idx(), n.x, n.y, n.z);
    }

    // Move each vertex one unit along its normal.
    let displacements: Vec<_> = mesh
        .vertices()
        .map(|vh| calc_vertex_normal(&mesh, vh))
        .collect();

    for (vh, n) in mesh.vertices().zip(displacements) {
        mesh.point_mut(vh).coords += n;
    }

    write_trimesh_obj(output, &mesh, None).expect("Failed to write output mesh");
    println!("Inflated mesh → {}", output);
}
