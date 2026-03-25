use crate::handle::{EdgeHandle, FaceHandle, HalfedgeHandle, VertexHandle};
use crate::error::MeshError;
use super::connectivity::Connectivity;

/// Triangle-specific topological operations on Connectivity.
///
/// These assume the mesh (or the relevant faces) are triangular.
/// Port of OpenMesh TriConnectivity and PolyConnectivity collapse.
impl Connectivity {
    /// Get the opposite vertex of a halfedge in its triangle face.
    /// The halfedge must belong to a triangle face.
    pub fn opposite_vh(&self, hh: HalfedgeHandle) -> VertexHandle {
        self.to_vertex_handle(self.next_halfedge_handle(hh))
    }

    /// Check if flipping the edge is topologically valid.
    /// Returns false for boundary edges or if the flip would create a duplicate edge.
    pub fn is_flip_ok(&self, eh: EdgeHandle) -> bool {
        // Boundary edges cannot be flipped
        if self.is_boundary_edge(eh) {
            return false;
        }

        let hh = eh.h0();
        let oh = eh.h1();

        // The two opposite vertices after flip
        let ah = self.to_vertex_handle(self.next_halfedge_handle(hh));
        let bh = self.to_vertex_handle(self.next_halfedge_handle(oh));

        if ah == bh {
            return false;
        }

        // Check if edge ah-bh already exists
        for vv in self.vv_cw_iter(ah) {
            if vv == bh {
                return false;
            }
        }

        true
    }

    /// Flip an edge in a triangle mesh.
    ///
    /// Before:         After:
    ///   va1             va1
    ///   / \             /|\
    ///  a2  a1          a2 a1
    /// /  a0 \    =>   / b0|a0\
    /// va0--vb0       va0  | vb0
    /// \  b0 /         \ a0|b0/
    ///  b2  b1          b2 b1
    ///   \ /             \|/
    ///   vb1             vb1
    pub fn flip(&mut self, eh: EdgeHandle) -> Result<(), MeshError> {
        debug_assert!(self.is_flip_ok(eh));

        let a0 = eh.h0();
        let b0 = eh.h1();

        let a1 = self.next_halfedge_handle(a0);
        let a2 = self.next_halfedge_handle(a1);

        let b1 = self.next_halfedge_handle(b0);
        let b2 = self.next_halfedge_handle(b1);

        let va0 = self.to_vertex_handle(a0);
        let va1 = self.to_vertex_handle(a1);

        let vb0 = self.to_vertex_handle(b0);
        let vb1 = self.to_vertex_handle(b1);

        let fa = self.face_handle(a0).ok_or(MeshError::BoundaryEdge)?;
        let fb = self.face_handle(b0).ok_or(MeshError::BoundaryEdge)?;

        self.set_vertex_handle(a0, va1);
        self.set_vertex_handle(b0, vb1);

        self.set_next_halfedge_handle(a0, a2);
        self.set_next_halfedge_handle(a2, b1);
        self.set_next_halfedge_handle(b1, a0);

        self.set_next_halfedge_handle(b0, b2);
        self.set_next_halfedge_handle(b2, a1);
        self.set_next_halfedge_handle(a1, b0);

        self.set_face_handle(a1, Some(fb));
        self.set_face_handle(b1, Some(fa));

        self.set_face_halfedge_handle(fa, a0);
        self.set_face_halfedge_handle(fb, b0);

        if self.vertex_halfedge_handle(va0) == Some(b0) {
            self.set_vertex_halfedge_handle(va0, Some(a1));
        }
        if self.vertex_halfedge_handle(vb0) == Some(a0) {
            self.set_vertex_halfedge_handle(vb0, Some(b1));
        }

        Ok(())
    }

    /// Check if collapsing the halfedge is topologically valid (for triangle meshes).
    ///
    /// Halfedge v0->v1: vertex v0 is removed, v1 survives.
    pub fn is_collapse_ok(&mut self, v0v1: HalfedgeHandle) -> bool {
        // Edge deleted?
        if self.edge_status(v0v1.edge()).is_deleted() {
            return false;
        }

        let v1v0 = v0v1.opposite();
        let v0 = self.to_vertex_handle(v1v0);
        let v1 = self.to_vertex_handle(v0v1);

        // Vertices deleted?
        if self.is_deleted_vertex(v0) || self.is_deleted_vertex(v1) {
            return false;
        }

        let mut vl = None;
        let mut vr = None;

        // Check left face (v0v1 side): edges v1-vl and vl-v0 must not both be boundary
        if !self.is_boundary_halfedge(v0v1) {
            let h1 = self.next_halfedge_handle(v0v1);
            let h2 = self.next_halfedge_handle(h1);
            vl = Some(self.to_vertex_handle(h1));

            if self.is_boundary_halfedge(h1.opposite())
                && self.is_boundary_halfedge(h2.opposite())
            {
                return false;
            }
        }

        // Check right face (v1v0 side): edges v0-vr and vr-v1 must not both be boundary
        if !self.is_boundary_halfedge(v1v0) {
            let h1 = self.next_halfedge_handle(v1v0);
            let h2 = self.next_halfedge_handle(h1);
            vr = Some(self.to_vertex_handle(h1));

            if self.is_boundary_halfedge(h1.opposite())
                && self.is_boundary_halfedge(h2.opposite())
            {
                return false;
            }
        }

        // vl and vr must be different (or both None, which shouldn't happen for valid mesh)
        if vl == vr {
            return false;
        }

        // One-ring intersection test: tag v1's neighbors, check v0's neighbors
        let ring_v0: Vec<VertexHandle> = self.vv_cw_iter(v0).collect();
        let ring_v1: Vec<VertexHandle> = self.vv_cw_iter(v1).collect();

        for &vv in &ring_v0 {
            self.vertex_status_mut(vv).set_tagged(false);
        }
        for &vv in &ring_v1 {
            self.vertex_status_mut(vv).set_tagged(true);
        }
        for &vv in &ring_v0 {
            if self.vertex_status(vv).is_tagged() && Some(vv) != vl && Some(vv) != vr {
                // Clean up tags before returning
                for &vv in &ring_v1 {
                    self.vertex_status_mut(vv).set_tagged(false);
                }
                return false;
            }
        }

        // Clean up tags
        for &vv in &ring_v1 {
            self.vertex_status_mut(vv).set_tagged(false);
        }

        // Edge between two boundary vertices must be a boundary edge
        if self.is_boundary_vertex(v0)
            && self.is_boundary_vertex(v1)
            && !self.is_boundary_halfedge(v0v1)
            && !self.is_boundary_halfedge(v1v0)
        {
            return false;
        }

        true
    }

    /// Collapse a halfedge: vertex `from_vertex(hh)` is removed,
    /// `to_vertex(hh)` survives.
    pub fn collapse(&mut self, hh: HalfedgeHandle) {
        let h0 = hh;
        let h1 = self.next_halfedge_handle(h0);
        let o0 = h0.opposite();
        let o1 = self.next_halfedge_handle(o0);

        // Remove edge
        self.collapse_edge(h0);

        // Remove degenerate loops (2-gons)
        if self.next_halfedge_handle(self.next_halfedge_handle(h1)) == h1 {
            self.collapse_loop(self.next_halfedge_handle(h1));
        }
        if self.next_halfedge_handle(self.next_halfedge_handle(o1)) == o1 {
            self.collapse_loop(o1);
        }
    }

    /// Internal: collapse edge (merge from-vertex into to-vertex).
    fn collapse_edge(&mut self, hh: HalfedgeHandle) {
        let h = hh;
        let hn = self.next_halfedge_handle(h);
        let hp = self.prev_halfedge_handle(h);

        let o = h.opposite();
        let on = self.next_halfedge_handle(o);
        let op = self.prev_halfedge_handle(o);

        let fh = self.face_handle(h);
        let fo = self.face_handle(o);

        let vh = self.to_vertex_handle(h);
        let vo = self.to_vertex_handle(o);

        // Redirect all incoming halfedges of vo to vh
        let incoming: Vec<HalfedgeHandle> = self.vih_cw_iter(vo).collect();
        for ih in incoming {
            self.set_vertex_handle(ih, vh);
        }

        // halfedge -> halfedge
        self.set_next_halfedge_handle(hp, hn);
        self.set_next_halfedge_handle(op, on);

        // face -> halfedge
        if let Some(f) = fh {
            self.set_face_halfedge_handle(f, hn);
        }
        if let Some(f) = fo {
            self.set_face_halfedge_handle(f, on);
        }

        // vertex -> halfedge
        if self.vertex_halfedge_handle(vh) == Some(o) {
            self.set_vertex_halfedge_handle(vh, Some(hn));
        }
        self.adjust_outgoing_halfedge(vh);
        self.set_vertex_halfedge_handle(vo, None); // isolated

        // Mark deleted
        self.edge_status_mut(h.edge()).set_deleted(true);
        self.vertex_status_mut(vo).set_deleted(true);
        self.halfedge_status_mut(h).set_deleted(true);
        self.halfedge_status_mut(o).set_deleted(true);
    }

    /// Internal: collapse a degenerate loop (2-gon) down to a single edge.
    fn collapse_loop(&mut self, hh: HalfedgeHandle) {
        let h0 = hh;
        let h1 = self.next_halfedge_handle(h0);

        let o0 = h0.opposite();
        let o1 = h1.opposite();

        let v0 = self.to_vertex_handle(h0);
        let v1 = self.to_vertex_handle(h1);

        let fh = self.face_handle(h0);
        let fo = self.face_handle(o0);

        debug_assert!(self.next_halfedge_handle(h1) == h0 && h1 != o0);

        // halfedge -> halfedge
        self.set_next_halfedge_handle(h1, self.next_halfedge_handle(o0));
        self.set_next_halfedge_handle(self.prev_halfedge_handle(o0), h1);

        // halfedge -> face
        if let Some(f) = fo {
            self.set_face_handle(h1, Some(f));
        } else {
            self.set_face_handle(h1, None);
        }

        // vertex -> halfedge
        self.set_vertex_halfedge_handle(v0, Some(h1));
        self.adjust_outgoing_halfedge(v0);
        self.set_vertex_halfedge_handle(v1, Some(o1));
        self.adjust_outgoing_halfedge(v1);

        // face -> halfedge
        if let Some(f) = fo {
            if self.face_halfedge_handle(f) == o0 {
                self.set_face_halfedge_handle(f, h1);
            }
        }

        // Delete stuff
        if let Some(f) = fh {
            self.face_status_mut(f).set_deleted(true);
        }
        self.edge_status_mut(h0.edge()).set_deleted(true);
        self.halfedge_status_mut(h0).set_deleted(true);
        self.halfedge_status_mut(o0).set_deleted(true);
    }

    /// Split an edge by inserting a new vertex. Works on triangle meshes.
    ///
    /// The new vertex `vh` is inserted on edge `eh`. Adjacent triangles are
    /// split into 2 each (2-to-4 for interior edges, 1-to-2 for boundary).
    pub fn split_edge(&mut self, eh: EdgeHandle, vh: VertexHandle) {
        let h0 = eh.h0();
        let o0 = eh.h1();

        let v2 = self.to_vertex_handle(o0);

        let e1 = self.new_edge(vh, v2);
        let t1 = e1.opposite();

        let f0 = self.face_handle(h0);
        let f3 = self.face_handle(o0);

        self.set_vertex_halfedge_handle(vh, Some(h0));
        self.set_vertex_handle(o0, vh);

        if let Some(f0) = f0 {
            // h0 side is not boundary
            let h1 = self.next_halfedge_handle(h0);
            let h2 = self.next_halfedge_handle(h1);
            let v1 = self.to_vertex_handle(h1);

            let e0 = self.new_edge(vh, v1);
            let t0 = e0.opposite();

            let f1 = self.new_face();
            self.set_face_halfedge_handle(f0, h0);
            self.set_face_halfedge_handle(f1, h2);

            self.set_face_handle(h1, Some(f0));
            self.set_face_handle(t0, Some(f0));
            self.set_face_handle(h0, Some(f0));

            self.set_face_handle(h2, Some(f1));
            self.set_face_handle(t1, Some(f1));
            self.set_face_handle(e0, Some(f1));

            self.set_next_halfedge_handle(h0, h1);
            self.set_next_halfedge_handle(h1, t0);
            self.set_next_halfedge_handle(t0, h0);

            self.set_next_halfedge_handle(e0, h2);
            self.set_next_halfedge_handle(h2, t1);
            self.set_next_halfedge_handle(t1, e0);
        } else {
            // h0 side is boundary
            let prev_h0 = self.prev_halfedge_handle(h0);
            self.set_next_halfedge_handle(prev_h0, t1);
            self.set_next_halfedge_handle(t1, h0);
        }

        if let Some(f3) = f3 {
            // o0 side is not boundary
            let o1 = self.next_halfedge_handle(o0);
            let o2 = self.next_halfedge_handle(o1);
            let v3 = self.to_vertex_handle(o1);

            let e2 = self.new_edge(vh, v3);
            let t2 = e2.opposite();

            let f2 = self.new_face();
            self.set_face_halfedge_handle(f2, o1);
            self.set_face_halfedge_handle(f3, o0);

            self.set_face_handle(o1, Some(f2));
            self.set_face_handle(t2, Some(f2));
            self.set_face_handle(e1, Some(f2));

            self.set_face_handle(o2, Some(f3));
            self.set_face_handle(o0, Some(f3));
            self.set_face_handle(e2, Some(f3));

            self.set_next_halfedge_handle(e1, o1);
            self.set_next_halfedge_handle(o1, t2);
            self.set_next_halfedge_handle(t2, e1);

            self.set_next_halfedge_handle(o0, e2);
            self.set_next_halfedge_handle(e2, o2);
            self.set_next_halfedge_handle(o2, o0);
        } else {
            // o0 side is boundary
            let next_o0 = self.next_halfedge_handle(o0);
            self.set_next_halfedge_handle(e1, next_o0);
            self.set_next_halfedge_handle(o0, e1);
            self.set_vertex_halfedge_handle(vh, Some(e1));
        }

        if self.vertex_halfedge_handle(v2) == Some(h0) {
            self.set_vertex_halfedge_handle(v2, Some(t1));
        }
    }

    /// Split a face by inserting a vertex at its center (1-to-3 split for triangles).
    pub fn split_face(&mut self, fh: FaceHandle, vh: VertexHandle) {
        // Get the three halfedges of the triangle face
        let hh0 = self.face_halfedge_handle(fh);
        let hh1 = self.next_halfedge_handle(hh0);
        let hh2 = self.next_halfedge_handle(hh1);

        let v0 = self.to_vertex_handle(hh0);
        let v1 = self.to_vertex_handle(hh1);
        let v2 = self.to_vertex_handle(hh2);

        // Create three new edges from vh to each triangle vertex
        let e0 = self.new_edge(vh, v0);
        let e1 = self.new_edge(vh, v1);
        let e2 = self.new_edge(vh, v2);

        let f1 = self.new_face();
        let f2 = self.new_face();

        // Set vertex halfedge
        self.set_vertex_halfedge_handle(vh, Some(e0));

        // Triangle 0 (original face): hh0 -> e0.opposite -> e1
        self.set_face_halfedge_handle(fh, hh0);
        self.set_face_handle(hh0, Some(fh));
        self.set_face_handle(e1, Some(fh));
        self.set_face_handle(e0.opposite(), Some(fh));
        self.set_next_halfedge_handle(hh0, e0.opposite());
        self.set_next_halfedge_handle(e0.opposite(), e1);
        self.set_next_halfedge_handle(e1, hh0);

        // Triangle 1: hh1 -> e1.opposite -> e2
        self.set_face_halfedge_handle(f1, hh1);
        self.set_face_handle(hh1, Some(f1));
        self.set_face_handle(e2, Some(f1));
        self.set_face_handle(e1.opposite(), Some(f1));
        self.set_next_halfedge_handle(hh1, e1.opposite());
        self.set_next_halfedge_handle(e1.opposite(), e2);
        self.set_next_halfedge_handle(e2, hh1);

        // Triangle 2: hh2 -> e2.opposite -> e0
        self.set_face_halfedge_handle(f2, hh2);
        self.set_face_handle(hh2, Some(f2));
        self.set_face_handle(e0, Some(f2));
        self.set_face_handle(e2.opposite(), Some(f2));
        self.set_next_halfedge_handle(hh2, e2.opposite());
        self.set_next_halfedge_handle(e2.opposite(), e0);
        self.set_next_halfedge_handle(e0, hh2);

        // Adjust outgoing halfedges
        self.adjust_outgoing_halfedge(v0);
        self.adjust_outgoing_halfedge(v1);
        self.adjust_outgoing_halfedge(v2);
    }
}
