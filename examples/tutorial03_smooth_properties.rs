//! Tutorial 03 — Smoothing with vertex properties.
//!
//! Port of OpenMesh Tutorial 03: same Laplacian smoothing as Tutorial 02 but
//! stores the per-vertex center-of-gravity in a `PropertyStore` vertex property
//! instead of a temporary Vec.

use std::env;

use infmesh::io::obj::{read_trimesh_obj, write_trimesh_obj};
use infmesh::property::PropertyStore;
use infmesh::VertexHandle;
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

    // Create a standalone property store and add a vertex property for center-of-gravity.
    let mut props = PropertyStore::new();
    let cog_prop = props.add::<VertexHandle, Point3<f64>>("v:cog", mesh.n_vertices());

    for _ in 0..iterations {
        // Phase 1: compute the center of gravity for each vertex.
        for vh in mesh.vertices() {
            if mesh.is_boundary_vertex(vh) {
                props.set(cog_prop, vh, *mesh.point(vh)).unwrap();
                continue;
            }
            let mut avg = Point3::origin();
            let mut count = 0u32;
            for vv in mesh.vv_ccw_iter(vh) {
                avg.coords += mesh.point(vv).coords;
                count += 1;
            }
            if count > 0 {
                avg.coords /= count as f64;
            }
            props.set(cog_prop, vh, avg).unwrap();
        }

        // Phase 2: apply stored positions.
        for vh in mesh.vertices() {
            let cog = props.get(cog_prop, vh).unwrap().clone();
            *mesh.point_mut(vh) = cog;
        }
    }

    write_trimesh_obj(output, &mesh, None).expect("Failed to write output mesh");
    println!("Smoothed {} iterations (using properties) → {}", iterations, output);
}
