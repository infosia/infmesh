//! Tutorial 02 — Basic Laplacian smoothing.
//!
//! Port of OpenMesh Tutorial 02: reads a triangle mesh, performs N iterations
//! of uniform Laplacian smoothing (skipping boundary vertices), and writes the
//! result.

use std::env;

use infmesh::io::obj::{read_trimesh_obj, write_trimesh_obj};
use nalgebra::Point3;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <input.obj> <iterations> <output.obj>", args[0]);
        std::process::exit(1);
    }

    let input = &args[1];
    let iterations: usize = args[2].parse().expect("iterations must be a number");
    let output = &args[3];

    let (mut mesh, _) = read_trimesh_obj(input).expect("Failed to read input mesh");
    println!(
        "Loaded: {} vertices, {} faces",
        mesh.n_vertices(),
        mesh.n_faces(),
    );

    // Smooth: for each non-boundary vertex, replace its position with the
    // average of its neighbours.
    for _ in 0..iterations {
        let n = mesh.n_vertices();
        let mut new_points = Vec::with_capacity(n);

        for vh in mesh.vertices() {
            if mesh.is_boundary_vertex(vh) {
                new_points.push(*mesh.point(vh));
                continue;
            }
            let mut cog = Point3::origin();
            let mut count = 0u32;
            for vv in mesh.vv_ccw_iter(vh) {
                cog.coords += mesh.point(vv).coords;
                count += 1;
            }
            if count > 0 {
                cog.coords /= count as infmesh::Scalar;
            }
            new_points.push(cog);
        }

        for (vh, pt) in mesh.vertices().zip(new_points) {
            *mesh.point_mut(vh) = pt;
        }
    }

    write_trimesh_obj(output, &mesh, None).expect("Failed to write output mesh");
    println!("Smoothed {} iterations → {}", iterations, output);
}
