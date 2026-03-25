use std::collections::HashMap;

use nalgebra::Point3;

use crate::Scalar;
use crate::handle::{EdgeHandle, VertexHandle};
use crate::trimesh::TriMesh;

/// Perform one iteration of Loop subdivision on a triangle mesh.
///
/// Algorithm (C.T. Loop, 1987):
/// 1. Compute new positions for existing vertices using neighbor averaging.
/// 2. Compute edge midpoint positions using the butterfly stencil.
/// 3. Split every edge at its midpoint.
/// 4. Flip edges that connect an old vertex to a new vertex.
///
/// Reference: C. T. Loop, "Smooth Subdivision Surfaces Based on Triangles", 1987.
pub fn loop_subdivide(mesh: &mut TriMesh) {
    let n_old_vertices = mesh.n_vertices();
    let n_old_edges = mesh.n_edges();

    // Phase 1: Compute new positions for old vertices
    let mut new_positions: Vec<Point3<Scalar>> = Vec::with_capacity(n_old_vertices);

    for vh in mesh.vertices() {
        if mesh.is_boundary_vertex(vh) {
            // Boundary vertex: average with boundary neighbors
            let mut boundary_neighbors = Vec::new();
            for hh in mesh.voh_cw_iter(vh) {
                if mesh.is_boundary_halfedge(hh) {
                    boundary_neighbors.push(mesh.to_vertex_handle(hh));
                }
            }
            if boundary_neighbors.len() == 2 {
                let p = mesh.point(vh);
                let p0 = mesh.point(boundary_neighbors[0]);
                let p1 = mesh.point(boundary_neighbors[1]);
                new_positions.push(Point3::from(
                    p.coords * 0.75 + p0.coords * 0.125 + p1.coords * 0.125,
                ));
            } else {
                new_positions.push(*mesh.point(vh));
            }
        } else {
            // Interior vertex: use Loop weights
            let n = mesh.valence_vertex(vh);
            let alpha = loop_alpha(n);
            let mut neighbor_sum = nalgebra::Vector3::zeros();
            for nvh in mesh.vv_cw_iter(vh) {
                neighbor_sum += mesh.point(nvh).coords;
            }
            let p = mesh.point(vh);
            new_positions.push(Point3::from(
                p.coords * (1.0 - alpha) + neighbor_sum * (alpha / n as Scalar),
            ));
        }
    }

    // Phase 2: Compute edge midpoint positions
    let mut edge_points: Vec<Point3<Scalar>> = Vec::with_capacity(n_old_edges);

    for ei in 0..n_old_edges {
        let eh = EdgeHandle::new(ei as u32);
        let h0 = eh.h0();
        let h1 = eh.h1();

        let v0 = mesh.to_vertex_handle(h0);
        let v1 = mesh.to_vertex_handle(h1);

        if mesh.is_boundary_edge(eh) {
            // Boundary edge: simple midpoint
            let p0 = mesh.point(v0);
            let p1 = mesh.point(v1);
            edge_points.push(Point3::from((p0.coords + p1.coords) * 0.5));
        } else {
            // Interior edge: 3/8 * (v0 + v1) + 1/8 * (vl + vr)
            let p0 = mesh.point(v0);
            let p1 = mesh.point(v1);
            let vl = mesh.to_vertex_handle(mesh.next_halfedge_handle(h0));
            let vr = mesh.to_vertex_handle(mesh.next_halfedge_handle(h1));
            let pl = mesh.point(vl);
            let pr = mesh.point(vr);
            edge_points.push(Point3::from(
                (p0.coords + p1.coords) * 0.375 + (pl.coords + pr.coords) * 0.125,
            ));
        }
    }

    // Phase 3: Split all old edges, inserting new vertices at edge midpoints.
    // We need to track which new vertex was created for each old edge.
    let mut edge_vertex_map: HashMap<usize, VertexHandle> = HashMap::new();

    for ei in 0..n_old_edges {
        let eh = EdgeHandle::new(ei as u32);
        let new_vh = mesh.add_vertex(edge_points[ei]);
        edge_vertex_map.insert(ei, new_vh);
        mesh.split_edge(eh, new_vh);
    }

    // Phase 4: Flip edges that connect an old vertex to a new vertex
    // After splitting, new edges were created. We need to flip edges
    // that connect old-old or new-new vertices incorrectly.
    // In Loop subdivision, we flip edges that were created by the split
    // and connect a new vertex to the opposite old vertex.
    for ei in n_old_edges..mesh.n_edges() {
        let eh = EdgeHandle::new(ei as u32);
        let h0 = eh.h0();
        let h1 = eh.h1();
        let v0 = mesh.to_vertex_handle(h0);
        let v1 = mesh.to_vertex_handle(h1);

        // An edge should be flipped if it connects an old vertex to a new vertex
        // and the edge is not on the boundary
        let v0_is_old = v0.idx() < n_old_vertices;
        let v1_is_old = v1.idx() < n_old_vertices;

        if v0_is_old != v1_is_old && !mesh.is_boundary_edge(eh) && mesh.is_flip_ok(eh) {
            mesh.flip(eh).expect("flip preconditions checked above");
        }
    }

    // Phase 5: Update old vertex positions
    for i in 0..n_old_vertices {
        let vh = VertexHandle::new(i as u32);
        *mesh.point_mut(vh) = new_positions[i];
    }
}

/// Compute the Loop alpha weight for a vertex of valence n.
/// alpha(n) = (40 - (3 + 2*cos(2*pi/n))^2) / 64
fn loop_alpha(n: usize) -> Scalar {
    let t = 3.0 + 2.0 * (2.0 * std::f32::consts::PI / n as Scalar).cos();
    (40.0 - t * t) / 64.0
}

/// Perform multiple iterations of Loop subdivision.
pub fn loop_subdivide_n(mesh: &mut TriMesh, iterations: usize) {
    for _ in 0..iterations {
        loop_subdivide(mesh);
    }
}
