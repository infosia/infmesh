use nalgebra::{Point3, Vector3};

use crate::handle::VertexHandle;
use crate::trimesh::TriMesh;

/// Laplacian smoothing configuration.
#[derive(Clone, Debug)]
pub struct SmoothOptions {
    /// Number of smoothing iterations.
    pub iterations: usize,
    /// Damping factor (0.0 = no smoothing, 1.0 = full Laplacian).
    /// Typical value: 0.5
    pub lambda: f64,
    /// If true, boundary vertices are kept fixed.
    pub fix_boundary: bool,
}

impl Default for SmoothOptions {
    fn default() -> Self {
        Self {
            iterations: 1,
            lambda: 0.5,
            fix_boundary: true,
        }
    }
}

/// Perform uniform Laplacian smoothing on a triangle mesh.
///
/// Each interior vertex is moved toward the centroid of its neighbors.
/// This is a Jacobi-style update: all new positions are computed from the
/// old positions before any vertex is moved.
///
/// Algorithm:
///   new_pos = old_pos + lambda * (centroid_of_neighbors - old_pos)
pub fn laplacian_smooth(mesh: &mut TriMesh, opts: &SmoothOptions) {
    let n = mesh.n_vertices();

    for _ in 0..opts.iterations {
        // Compute new positions (Jacobi: read old, write new)
        let mut new_positions: Vec<Point3<f64>> = Vec::with_capacity(n);

        for vi in 0..n {
            let vh = VertexHandle::new(vi as u32);

            // Skip boundary vertices if requested
            if opts.fix_boundary && mesh.is_boundary_vertex(vh) {
                new_positions.push(*mesh.point(vh));
                continue;
            }

            // Compute centroid of neighbors (umbrella operator)
            let mut sum = Vector3::zeros();
            let mut count = 0usize;
            for nvh in mesh.vv_cw_iter(vh) {
                sum += mesh.point(nvh).coords;
                count += 1;
            }

            if count == 0 {
                new_positions.push(*mesh.point(vh));
                continue;
            }

            let centroid = sum / count as f64;
            let p = mesh.point(vh).coords;
            let new_p = p + opts.lambda * (centroid - p);
            new_positions.push(Point3::from(new_p));
        }

        // Apply new positions
        for vi in 0..n {
            let vh = VertexHandle::new(vi as u32);
            *mesh.point_mut(vh) = new_positions[vi];
        }
    }
}

/// Taubin smoothing: alternating positive/negative lambda to avoid shrinkage.
///
/// Each iteration consists of two Laplacian steps:
///   1. Smooth with +lambda (shrink)
///   2. Smooth with -mu (expand), where mu > lambda
///
/// This preserves volume better than plain Laplacian smoothing.
pub fn taubin_smooth(mesh: &mut TriMesh, iterations: usize, lambda: f64, mu: f64, fix_boundary: bool) {
    let opts_shrink = SmoothOptions {
        iterations: 1,
        lambda,
        fix_boundary,
    };
    let opts_expand = SmoothOptions {
        iterations: 1,
        lambda: -mu,
        fix_boundary,
    };

    for _ in 0..iterations {
        laplacian_smooth(mesh, &opts_shrink);
        laplacian_smooth(mesh, &opts_expand);
    }
}
