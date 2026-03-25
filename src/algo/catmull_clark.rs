use std::collections::HashMap;

use nalgebra::Point3;

use crate::Scalar;
use crate::handle::{FaceHandle, VertexHandle};
use crate::polymesh::PolyMesh;

/// Perform one iteration of Catmull-Clark subdivision on a polygon mesh.
///
/// Algorithm (E. Catmull & J. Clark, 1978):
/// 1. Compute face points (centroids of each face).
/// 2. Compute edge points from adjacent face points and edge endpoints.
/// 3. Compute new positions for original vertices.
/// 4. Build a new mesh with quad faces connecting face points, edge points,
///    and repositioned original vertices.
///
/// This implementation snapshots the old mesh, builds a new mesh from scratch,
/// then swaps it in to avoid handle invalidation issues.
pub fn catmull_clark_subdivide(mesh: &mut PolyMesh) {
    // ---------------------------------------------------------------
    // Phase 1: Snapshot old mesh
    // ---------------------------------------------------------------

    // Collect live faces: face_idx -> Vec<VertexHandle>
    let mut face_verts: HashMap<usize, Vec<VertexHandle>> = HashMap::new();
    for fh in mesh.faces() {
        if mesh.is_deleted_face(fh) {
            continue;
        }
        face_verts.insert(fh.idx(), mesh.face_vertices(fh));
    }

    // Collect live edges: edge_idx -> (v0, v1)
    let mut edge_endpoints: HashMap<usize, (VertexHandle, VertexHandle)> = HashMap::new();
    for eh in mesh.edges() {
        if mesh.is_deleted_edge(eh) {
            continue;
        }
        let v0 = mesh.to_vertex_handle(eh.h0());
        let v1 = mesh.to_vertex_handle(eh.h1());
        edge_endpoints.insert(eh.idx(), (v0, v1));
    }

    // Collect live vertices
    let mut vertex_positions: HashMap<usize, Point3<Scalar>> = HashMap::new();
    let mut vertex_is_boundary: HashMap<usize, bool> = HashMap::new();
    for vh in mesh.vertices() {
        if mesh.is_deleted_vertex(vh) {
            continue;
        }
        vertex_positions.insert(vh.idx(), *mesh.point(vh));
        vertex_is_boundary.insert(vh.idx(), mesh.is_boundary_vertex(vh));
    }

    // Build edge adjacency: edge_idx -> Vec<face_idx>
    let mut edge_faces: HashMap<usize, Vec<usize>> = HashMap::new();
    for (&fi, _) in &face_verts {
        let fh = FaceHandle::new(fi as u32);
        for eh in mesh.fe_ccw_iter(fh) {
            edge_faces.entry(eh.idx()).or_default().push(fi);
        }
    }

    // Build vertex -> adjacent face indices
    let mut vertex_adj_faces: HashMap<usize, Vec<usize>> = HashMap::new();
    for vh in mesh.vertices() {
        if mesh.is_deleted_vertex(vh) {
            continue;
        }
        let faces: Vec<usize> = mesh
            .vf_cw_iter(vh)
            .filter(|&fh| !mesh.is_deleted_face(fh))
            .map(|fh| fh.idx())
            .collect();
        vertex_adj_faces.insert(vh.idx(), faces);
    }

    // Build vertex -> adjacent edge indices
    let mut vertex_adj_edges: HashMap<usize, Vec<usize>> = HashMap::new();
    for vh in mesh.vertices() {
        if mesh.is_deleted_vertex(vh) {
            continue;
        }
        let edges: Vec<usize> = mesh
            .ve_cw_iter(vh)
            .filter(|&eh| !mesh.is_deleted_edge(eh))
            .map(|eh| eh.idx())
            .collect();
        vertex_adj_edges.insert(vh.idx(), edges);
    }

    // Build vertex -> boundary neighbors (for boundary vertices)
    let mut vertex_boundary_neighbors: HashMap<usize, Vec<VertexHandle>> = HashMap::new();
    for vh in mesh.vertices() {
        if mesh.is_deleted_vertex(vh) || !mesh.is_boundary_vertex(vh) {
            continue;
        }
        let mut bnd_neighbors = Vec::new();
        for hh in mesh.voh_cw_iter(vh) {
            if mesh.is_boundary_halfedge(hh) {
                bnd_neighbors.push(mesh.to_vertex_handle(hh));
            }
        }
        vertex_boundary_neighbors.insert(vh.idx(), bnd_neighbors);
    }

    // Snapshot the halfedge lookup: for each pair of adjacent vertices in a face,
    // find the edge index. We need this to map face-edge relationships when building
    // the new mesh.
    let mut pair_to_edge: HashMap<(usize, usize), usize> = HashMap::new();
    for (&fi, verts) in &face_verts {
        let _ = fi;
        let n = verts.len();
        for i in 0..n {
            let va = verts[i];
            let vb = verts[(i + 1) % n];
            if let Some(hh) = mesh.find_halfedge(va, vb) {
                pair_to_edge.insert((va.idx(), vb.idx()), hh.edge().idx());
            }
        }
    }

    // ---------------------------------------------------------------
    // Phase 2: Compute new positions
    // ---------------------------------------------------------------

    // Face points: face_idx -> Point3
    let mut face_points: HashMap<usize, Point3<Scalar>> = HashMap::new();
    for (&fi, verts) in &face_verts {
        let mut sum = nalgebra::Vector3::zeros();
        for v in verts {
            sum += vertex_positions[&v.idx()].coords;
        }
        let n = verts.len() as Scalar;
        face_points.insert(fi, Point3::from(sum / n));
    }

    // Edge points: edge_idx -> Point3
    let mut edge_points_map: HashMap<usize, Point3<Scalar>> = HashMap::new();
    for (&ei, &(v0, v1)) in &edge_endpoints {
        let p0 = vertex_positions[&v0.idx()];
        let p1 = vertex_positions[&v1.idx()];
        let adj = edge_faces.get(&ei);
        let is_boundary = adj.map_or(true, |fs| fs.len() < 2);
        if is_boundary {
            edge_points_map.insert(ei, Point3::from((p0.coords + p1.coords) * 0.5));
        } else {
            let fs = adj.unwrap();
            let fp0 = face_points[&fs[0]];
            let fp1 = face_points[&fs[1]];
            edge_points_map.insert(
                ei,
                Point3::from((p0.coords + p1.coords + fp0.coords + fp1.coords) * 0.25),
            );
        }
    }

    // Vertex points: vertex_idx -> Point3
    let mut new_vertex_positions: HashMap<usize, Point3<Scalar>> = HashMap::new();
    for (&vi, &pos) in &vertex_positions {
        let is_boundary = vertex_is_boundary[&vi];
        if is_boundary {
            let bnd_neighbors = vertex_boundary_neighbors.get(&vi);
            if let Some(neighbors) = bnd_neighbors {
                if neighbors.len() == 2 {
                    let b0 = vertex_positions[&neighbors[0].idx()];
                    let b1 = vertex_positions[&neighbors[1].idx()];
                    new_vertex_positions.insert(
                        vi,
                        Point3::from(b0.coords * 0.25 + pos.coords * 0.5 + b1.coords * 0.25),
                    );
                } else {
                    new_vertex_positions.insert(vi, pos);
                }
            } else {
                new_vertex_positions.insert(vi, pos);
            }
        } else {
            let adj_faces = &vertex_adj_faces[&vi];
            let adj_edges = &vertex_adj_edges[&vi];
            let n = adj_faces.len();
            if n == 0 {
                new_vertex_positions.insert(vi, pos);
                continue;
            }

            // F = average of adjacent face points
            let mut f_sum = nalgebra::Vector3::zeros();
            for &fi in adj_faces {
                f_sum += face_points[&fi].coords;
            }
            let f_avg = f_sum / n as Scalar;

            // R = average of midpoints of adjacent edges
            let mut r_sum = nalgebra::Vector3::zeros();
            let mut r_count = 0;
            for &ei in adj_edges {
                if let Some(&(v0, v1)) = edge_endpoints.get(&ei) {
                    let mid =
                        (vertex_positions[&v0.idx()].coords + vertex_positions[&v1.idx()].coords)
                            * 0.5;
                    r_sum += mid;
                    r_count += 1;
                }
            }
            let r_avg = if r_count > 0 {
                r_sum / r_count as Scalar
            } else {
                nalgebra::Vector3::zeros()
            };

            let n_s = n as Scalar;
            let new_pos = (f_avg + r_avg * 2.0 + pos.coords * (n_s - 3.0)) / n_s;
            new_vertex_positions.insert(vi, Point3::from(new_pos));
        }
    }

    // ---------------------------------------------------------------
    // Phase 3: Build new mesh
    // ---------------------------------------------------------------

    let mut new_mesh = PolyMesh::new();

    // Add face point vertices: old_face_idx -> new VertexHandle
    let mut face_vh_map: HashMap<usize, VertexHandle> = HashMap::new();
    for (&fi, fp) in &face_points {
        let vh = new_mesh.add_vertex(*fp);
        face_vh_map.insert(fi, vh);
    }

    // Add edge point vertices: old_edge_idx -> new VertexHandle
    let mut edge_vh_map: HashMap<usize, VertexHandle> = HashMap::new();
    for (&ei, ep) in &edge_points_map {
        let vh = new_mesh.add_vertex(*ep);
        edge_vh_map.insert(ei, vh);
    }

    // Add repositioned original vertices: old_vertex_idx -> new VertexHandle
    let mut vert_vh_map: HashMap<usize, VertexHandle> = HashMap::new();
    for (&vi, pos) in &new_vertex_positions {
        let vh = new_mesh.add_vertex(*pos);
        vert_vh_map.insert(vi, vh);
    }

    // For each old face, create quads
    for (&fi, verts) in &face_verts {
        let n = verts.len();
        let face_v = face_vh_map[&fi];

        for i in 0..n {
            let v_curr = verts[i];
            let v_next = verts[(i + 1) % n];
            let v_prev = verts[(n + i - 1) % n];

            // Edge between v_curr and v_next
            let edge_next = pair_to_edge
                .get(&(v_curr.idx(), v_next.idx()))
                .or_else(|| pair_to_edge.get(&(v_next.idx(), v_curr.idx())))
                .expect("edge between consecutive face vertices must exist");

            // Edge between v_prev and v_curr
            let edge_prev = pair_to_edge
                .get(&(v_prev.idx(), v_curr.idx()))
                .or_else(|| pair_to_edge.get(&(v_curr.idx(), v_prev.idx())))
                .expect("edge between consecutive face vertices must exist");

            let new_v_curr = vert_vh_map[&v_curr.idx()];
            let new_edge_next = edge_vh_map[edge_next];
            let new_edge_prev = edge_vh_map[edge_prev];

            new_mesh
                .add_face(&[new_v_curr, new_edge_next, face_v, new_edge_prev])
                .expect("failed to add subdivision quad face");
        }
    }

    // Swap
    *mesh = new_mesh;
}

/// Perform multiple iterations of Catmull-Clark subdivision.
pub fn catmull_clark_subdivide_n(mesh: &mut PolyMesh, iterations: usize) {
    for _ in 0..iterations {
        catmull_clark_subdivide(mesh);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Point3;

    fn make_cube() -> PolyMesh {
        let mut mesh = PolyMesh::new();
        let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));
        let v4 = mesh.add_vertex(Point3::new(0.0, 0.0, 1.0));
        let v5 = mesh.add_vertex(Point3::new(1.0, 0.0, 1.0));
        let v6 = mesh.add_vertex(Point3::new(1.0, 1.0, 1.0));
        let v7 = mesh.add_vertex(Point3::new(0.0, 1.0, 1.0));
        mesh.add_face(&[v0, v1, v2, v3]).unwrap();
        mesh.add_face(&[v5, v4, v7, v6]).unwrap();
        mesh.add_face(&[v4, v0, v3, v7]).unwrap();
        mesh.add_face(&[v1, v5, v6, v2]).unwrap();
        mesh.add_face(&[v4, v5, v1, v0]).unwrap();
        mesh.add_face(&[v3, v2, v6, v7]).unwrap();
        mesh
    }

    #[test]
    fn test_catmull_clark_cube() {
        let mut mesh = make_cube();
        catmull_clark_subdivide(&mut mesh);

        // A cube with 6 quad faces should produce 24 quad faces (4 per original face)
        let live_faces: usize = mesh
            .faces()
            .filter(|&fh| !mesh.is_deleted_face(fh))
            .count();
        assert_eq!(live_faces, 24, "6 quads * 4 = 24 faces after subdivision");

        // 8 original verts + 6 face points + 12 edge points = 26 vertices
        let live_verts: usize = mesh
            .vertices()
            .filter(|&vh| !mesh.is_deleted_vertex(vh))
            .count();
        assert_eq!(live_verts, 26, "8 + 6 + 12 = 26 vertices after subdivision");
    }

    #[test]
    fn test_catmull_clark_multiple_iterations() {
        let mut mesh = make_cube();
        catmull_clark_subdivide_n(&mut mesh, 2);

        // After 2 iterations: 24 * 4 = 96 faces
        let live_faces: usize = mesh
            .faces()
            .filter(|&fh| !mesh.is_deleted_face(fh))
            .count();
        assert_eq!(live_faces, 96);
    }

    #[test]
    fn test_catmull_clark_single_quad() {
        let mut mesh = PolyMesh::new();
        let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));
        mesh.add_face(&[v0, v1, v2, v3]).unwrap();

        catmull_clark_subdivide(&mut mesh);

        // 1 face -> 4 faces, 4 original verts + 1 face point + 4 edge points = 9 verts
        let live_faces: usize = mesh
            .faces()
            .filter(|&fh| !mesh.is_deleted_face(fh))
            .count();
        assert_eq!(live_faces, 4);

        let live_verts: usize = mesh
            .vertices()
            .filter(|&vh| !mesh.is_deleted_vertex(vh))
            .count();
        assert_eq!(live_verts, 9);
    }
}
