use std::collections::BinaryHeap;
use std::cmp::Ordering;

use nalgebra::{Matrix4, Vector4};

use crate::Scalar;
use crate::handle::HalfedgeHandle;
use crate::trimesh::TriMesh;
use crate::geometry;

/// Quadric error metric for mesh decimation.
///
/// Each vertex stores a 4x4 symmetric matrix Q. The error of placing
/// a vertex at position v is v^T * Q * v. When an edge is collapsed,
/// the quadrics of the two endpoints are summed.
///
/// Reference: Garland & Heckbert, "Surface Simplification Using Quadric Error Metrics", 1997.
#[derive(Clone, Debug)]
struct Quadric {
    m: Matrix4<Scalar>,
}

impl Default for Quadric {
    fn default() -> Self {
        Self {
            m: Matrix4::zeros(),
        }
    }
}

impl Quadric {
    /// Create a quadric from a plane equation ax + by + cz + d = 0.
    fn from_plane(a: Scalar, b: Scalar, c: Scalar, d: Scalar) -> Self {
        let v = Vector4::new(a, b, c, d);
        Self { m: v * v.transpose() }
    }

    /// Evaluate the error at position (x, y, z).
    fn evaluate(&self, x: Scalar, y: Scalar, z: Scalar) -> Scalar {
        let v = Vector4::new(x, y, z, 1.0);
        (v.transpose() * &self.m * v)[(0, 0)]
    }

    /// Sum two quadrics.
    fn add(&self, other: &Quadric) -> Quadric {
        Quadric {
            m: self.m + other.m,
        }
    }
}

/// A candidate edge collapse with its priority (error).
#[derive(Clone, Debug)]
struct CollapseCandidate {
    halfedge: HalfedgeHandle,
    error: Scalar,
}

impl PartialEq for CollapseCandidate {
    fn eq(&self, other: &Self) -> bool {
        // Treat NaN as equal to NaN for BinaryHeap consistency
        if self.error.is_nan() && other.error.is_nan() {
            return true;
        }
        self.error == other.error
    }
}

impl Eq for CollapseCandidate {}

impl PartialOrd for CollapseCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CollapseCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap: reverse ordering so smallest error has highest priority.
        // NaN errors get lowest priority (Greater = lower priority in max-heap).
        match (self.error.is_nan(), other.error.is_nan()) {
            (true, true) => Ordering::Equal,
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => other.error.partial_cmp(&self.error).unwrap_or(Ordering::Equal),
        }
    }
}

/// Decimate a triangle mesh using quadric error metrics.
///
/// Reduces the mesh to at most `target_vertices` vertices by iteratively
/// collapsing the edge with the smallest quadric error.
///
/// Returns the number of collapses performed.
pub fn decimate(mesh: &mut TriMesh, target_vertices: usize) -> usize {
    let nv = mesh.n_vertices();
    if nv <= target_vertices {
        return 0;
    }

    // Step 1: Compute initial quadrics for each vertex
    let mut quadrics: Vec<Quadric> = vec![Quadric::default(); nv];

    for fh in mesh.faces() {
        let n = geometry::calc_face_normal(mesh, fh);
        // Get any vertex of the face to compute d
        let hh = mesh.face_halfedge_handle(fh);
        let vh = mesh.to_vertex_handle(hh);
        let p = mesh.point(vh);
        let d = -(n.x * p.x + n.y * p.y + n.z * p.z);
        let q = Quadric::from_plane(n.x, n.y, n.z, d);

        // Add to all vertices of this face
        for fvh in mesh.fv_ccw_iter(fh) {
            quadrics[fvh.idx()] = quadrics[fvh.idx()].add(&q);
        }
    }

    // Step 2: Build priority queue of edge collapses
    let mut heap: BinaryHeap<CollapseCandidate> = BinaryHeap::new();

    for eh in mesh.edges() {
        let h0 = eh.h0();
        let h1 = eh.h1();
        let v0 = mesh.to_vertex_handle(h1); // from vertex
        let v1 = mesh.to_vertex_handle(h0); // to vertex

        // Error of collapsing to v1's position
        let p1 = mesh.point(v1);
        let combined = quadrics[v0.idx()].add(&quadrics[v1.idx()]);
        let error = combined.evaluate(p1.x, p1.y, p1.z);

        heap.push(CollapseCandidate {
            halfedge: h0,
            error,
        });
    }

    // Step 3: Iteratively collapse lowest-error edges
    let mut collapses = 0;
    let mut current_vertices = nv;

    while current_vertices > target_vertices {
        let candidate = match heap.pop() {
            Some(c) => c,
            None => break,
        };

        // Skip NaN-error candidates
        if candidate.error.is_nan() {
            continue;
        }

        let hh = candidate.halfedge;

        // Skip if edge is already deleted
        if mesh.is_deleted_edge(hh.edge()) {
            continue;
        }

        // Skip if collapse would create invalid topology
        if !mesh.is_collapse_ok(hh) {
            continue;
        }

        let v_from = mesh.from_vertex_handle(hh);
        let v_to = mesh.to_vertex_handle(hh);

        // Update quadric of surviving vertex
        let new_quadric = quadrics[v_from.idx()].add(&quadrics[v_to.idx()]);
        quadrics[v_to.idx()] = new_quadric;

        // Perform collapse
        mesh.collapse(hh);
        collapses += 1;
        current_vertices -= 1;

        // Re-insert affected edges into the heap
        // (neighbors of v_to now have new errors)
        let neighbors: Vec<HalfedgeHandle> = mesh.voh_cw_iter(v_to).collect();
        for oh in neighbors {
            if mesh.is_deleted_halfedge(oh) {
                continue;
            }
            let v_neighbor = mesh.to_vertex_handle(oh);
            if mesh.is_deleted_vertex(v_neighbor) {
                continue;
            }
            let p = mesh.point(v_to);
            let combined = quadrics[v_to.idx()].add(&quadrics[v_neighbor.idx()]);
            let error = combined.evaluate(p.x, p.y, p.z);
            heap.push(CollapseCandidate {
                halfedge: oh,
                error,
            });
        }
    }

    // Step 4: Clean up deleted elements
    mesh.garbage_collection();

    collapses
}

/// Decimate to a target face count.
pub fn decimate_to_faces(mesh: &mut TriMesh, target_faces: usize) -> usize {
    // Euler formula for closed triangle mesh: F ≈ 2V - 4
    // So target_vertices ≈ (target_faces + 4) / 2
    // For open meshes this is approximate, but good enough as a target
    let target_v = (target_faces + 4) / 2;
    decimate(mesh, target_v)
}
