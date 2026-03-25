use std::collections::{HashMap, HashSet};

use crate::handle::{EdgeHandle, FaceHandle, VertexHandle};
use crate::error::MeshError;
use crate::polymesh::PolyMesh;
use super::connectivity::Connectivity;

// ── Result types ─────────────────────────────────────────────────────

pub struct InsertEdgeResult {
    pub face_a: FaceHandle,
    pub face_b: FaceHandle,
    pub edge: EdgeHandle,
}

pub struct ExtrudeResult {
    pub top_faces: Vec<FaceHandle>,
    pub new_vertices: Vec<VertexHandle>,
}

pub struct LoopCutResult {
    pub new_edges: Vec<EdgeHandle>,
    pub new_faces: Vec<FaceHandle>,
}

pub struct SplitEdgeResult {
    pub new_vertex: VertexHandle,
    pub new_edges: Vec<EdgeHandle>,
    pub new_faces: Vec<FaceHandle>,
}

pub struct ExtrudeEdgesResult {
    pub new_vertices: Vec<VertexHandle>,
    pub new_faces: Vec<FaceHandle>,
}

// ── Helpers ──────────────────────────────────────────────────────────

/// Canonical edge key (order-independent) for use as HashMap key.
fn edge_key(a: VertexHandle, b: VertexHandle) -> (VertexHandle, VertexHandle) {
    if a.idx() <= b.idx() { (a, b) } else { (b, a) }
}

// ── Connectivity operations ──────────────────────────────────────────

impl Connectivity {
    /// Collect ordered vertices of a face (CCW winding).
    pub fn face_vertices(&self, fh: FaceHandle) -> Vec<VertexHandle> {
        self.fv_ccw_iter(fh).collect()
    }

    /// Find the edge ring passing through the given edge.
    ///
    /// An edge ring is a sequence of edges that cross faces by jumping to the
    /// "opposite" edge (N/2 steps around an N-sided face). This is the standard
    /// Blender-style edge ring selection.
    ///
    /// Returns all edges in the ring, including the start edge.
    pub fn find_edge_ring(&self, eh: EdgeHandle) -> Vec<EdgeHandle> {
        let mut result = vec![eh];
        let mut seen: HashSet<EdgeHandle> = HashSet::new();
        seen.insert(eh);

        // Walk in both directions (through each face adjacent to the start edge)
        for side in 0..2u32 {
            let hh = eh.halfedge(side);
            let Some(first_face) = self.face_handle(hh) else {
                continue; // boundary side
            };

            let mut current_face = first_face;
            let mut current_eh = eh;

            loop {
                // Get ordered edges of this face
                let face_edges: Vec<EdgeHandle> = self.fe_ccw_iter(current_face).collect();
                let n = face_edges.len();
                if n < 3 { break; }

                let Some(pos) = face_edges.iter().position(|&e| e == current_eh) else {
                    break;
                };

                // Opposite edge: N/2 steps away (exact for even N, best approx for odd)
                let opp_eh = face_edges[(pos + n / 2) % n];

                if !seen.insert(opp_eh) { break; } // already visited

                result.push(opp_eh);

                // Cross to the face on the other side of the opposite edge
                let opp_h0 = opp_eh.h0();
                let opp_h1 = opp_eh.h1();
                let next_face = if self.face_handle(opp_h0) == Some(current_face) {
                    self.face_handle(opp_h1)
                } else {
                    self.face_handle(opp_h0)
                };
                let Some(next_face) = next_face else { break; }; // boundary

                current_face = next_face;
                current_eh = opp_eh;
            }
        }

        result
    }

    /// Find a face that contains both vertices `a` and `b`.
    pub fn find_shared_face(&self, a: VertexHandle, b: VertexHandle) -> Option<FaceHandle> {
        let faces_a: HashSet<FaceHandle> = self.vf_cw_iter(a).collect();
        for fh in self.vf_cw_iter(b) {
            if faces_a.contains(&fh) {
                return Some(fh);
            }
        }
        None
    }

    /// Insert an edge across a face, splitting it into two faces.
    ///
    /// `va` and `vb` must be non-adjacent vertices on the given face.
    /// The face must have at least 4 vertices (can't split a triangle).
    pub fn insert_edge(
        &mut self,
        fh: FaceHandle,
        va: VertexHandle,
        vb: VertexHandle,
    ) -> Result<InsertEdgeResult, MeshError> {
        let verts = self.face_vertices(fh);
        let n = verts.len();
        if n < 4 {
            return Err(MeshError::DegenerateFace);
        }

        let idx_a = verts.iter().position(|&v| v == va)
            .ok_or(MeshError::DegenerateFace)?;
        let idx_b = verts.iter().position(|&v| v == vb)
            .ok_or(MeshError::DegenerateFace)?;

        // Validate non-adjacent
        let diff = (idx_b as isize - idx_a as isize).unsigned_abs() % n;
        if diff <= 1 || diff >= n - 1 {
            return Err(MeshError::DegenerateFace); // adjacent or same vertex
        }

        // Delete the old face
        self.delete_face(fh, false);

        // Build two new vertex subsequences
        // Face 1: from idx_a around to idx_b (inclusive)
        let mut verts_1 = Vec::new();
        let mut i = idx_a;
        loop {
            verts_1.push(verts[i]);
            if i == idx_b { break; }
            i = (i + 1) % n;
        }

        // Face 2: from idx_b around to idx_a (inclusive)
        let mut verts_2 = Vec::new();
        i = idx_b;
        loop {
            verts_2.push(verts[i]);
            if i == idx_a { break; }
            i = (i + 1) % n;
        }

        // Add new faces
        let fh_a = self.add_face(&verts_1)?;
        let fh_b = self.add_face(&verts_2)?;

        // Find the new edge between va and vb
        let hh = self.find_halfedge(va, vb)
            .ok_or(MeshError::DegenerateFace)?;

        Ok(InsertEdgeResult {
            face_a: fh_a,
            face_b: fh_b,
            edge: hh.edge(),
        })
    }
}

// ── PolyMesh operations ──────────────────────────────────────────────

impl PolyMesh {
    /// Extrude selected faces: duplicate vertices, create side quads, return
    /// the new top faces and new vertex handles for immediate translation.
    ///
    /// After extrusion, the top faces sit at the same position as the originals.
    /// The caller should translate `new_vertices` by the desired offset (e.g.
    /// along the average face normal) to complete the extrusion.
    pub fn extrude_faces(
        &mut self,
        faces: &[FaceHandle],
    ) -> Result<ExtrudeResult, MeshError> {
        if faces.is_empty() {
            return Err(MeshError::DegenerateFace);
        }

        let face_set: HashSet<FaceHandle> = faces.iter().copied().collect();

        // Phase 1: Analyze — collect face data, classify boundary vs inner edges
        struct FaceInfo {
            verts: Vec<VertexHandle>,
        }
        let mut face_data: Vec<FaceInfo> = Vec::new();
        let mut all_verts: HashSet<VertexHandle> = HashSet::new();
        let mut boundary_edges: Vec<(VertexHandle, VertexHandle)> = Vec::new();
        let mut boundary_edge_set: HashSet<(VertexHandle, VertexHandle)> = HashSet::new();

        for &fh in faces {
            let verts = self.face_vertices(fh);
            let n = verts.len();
            for i in 0..n {
                let v_src = verts[i];
                let v_dst = verts[(i + 1) % n];
                all_verts.insert(v_src);

                // Check if the other face of this edge is also selected
                let key = edge_key(v_src, v_dst);
                if let Some(hh) = self.find_halfedge(v_src, v_dst) {
                    let eh = hh.edge();
                    let f0 = self.face_handle(eh.h0());
                    let f1 = self.face_handle(eh.h1());
                    let other_selected = [f0, f1].iter()
                        .filter_map(|f| *f)
                        .any(|f| f != fh && face_set.contains(&f));
                    if !other_selected {
                        if boundary_edge_set.insert(key) {
                            boundary_edges.push((v_src, v_dst));
                        }
                    }
                }
            }
            face_data.push(FaceInfo { verts });
        }

        // Phase 2: Duplicate vertices
        let mut old_to_new: HashMap<VertexHandle, VertexHandle> = HashMap::new();
        let mut new_vertices: Vec<VertexHandle> = Vec::new();

        for &v in &all_verts {
            let pos = *self.point(v);
            let new_v = self.add_vertex(pos);
            old_to_new.insert(v, new_v);
            new_vertices.push(new_v);
        }

        // Phase 3: Remove old faces
        for &fh in faces {
            self.delete_face(fh, false);
        }

        // Phase 4: Create top faces with new vertices
        let mut top_faces: Vec<FaceHandle> = Vec::new();
        for info in &face_data {
            let new_verts: Vec<VertexHandle> = info.verts.iter()
                .map(|v| old_to_new[v])
                .collect();
            let new_fh = self.add_face(&new_verts)?;
            top_faces.push(new_fh);
        }

        // Phase 5: Create side quads along boundary edges
        for &(v_src, v_dst) in &boundary_edges {
            let new_src = old_to_new[&v_src];
            let new_dst = old_to_new[&v_dst];
            // Quad winding: old_src -> old_dst -> new_dst -> new_src
            let _ = self.add_face(&[v_src, v_dst, new_dst, new_src]);
        }

        Ok(ExtrudeResult {
            top_faces,
            new_vertices,
        })
    }

    /// Perform a loop cut along the edge ring that passes through the given edge.
    ///
    /// Splits each ring edge at its midpoint and rebuilds adjacent faces.
    /// For quads crossed by 2 ring edges, the face is split into 2 sub-faces.
    pub fn loop_cut(
        &mut self,
        eh: EdgeHandle,
    ) -> Result<LoopCutResult, MeshError> {
        // Find the edge ring
        let ring = self.find_edge_ring(eh);
        if ring.is_empty() {
            return Err(MeshError::DegenerateFace);
        }

        // Collect ring edges as sorted vertex-pair keys and their EdgeHandles
        let ring_keys: HashSet<(VertexHandle, VertexHandle)> = ring.iter()
            .map(|&e| {
                let v0 = self.from_vertex_handle(e.h0());
                let v1 = self.to_vertex_handle(e.h0());
                edge_key(v0, v1)
            })
            .collect();

        // Collect all faces adjacent to ring edges
        let mut affected_faces: HashSet<FaceHandle> = HashSet::new();
        for &e in &ring {
            if let Some(f) = self.face_handle(e.h0()) {
                affected_faces.insert(f);
            }
            if let Some(f) = self.face_handle(e.h1()) {
                affected_faces.insert(f);
            }
        }

        if affected_faces.is_empty() {
            return Err(MeshError::DegenerateFace);
        }

        // Create midpoint vertices for each ring edge
        let mut edge_mids: HashMap<(VertexHandle, VertexHandle), VertexHandle> = HashMap::new();
        for &(va, vb) in &ring_keys {
            let pos_a = self.point(va).clone();
            let pos_b = self.point(vb).clone();
            let mid_pos = nalgebra::Point3::new(
                (pos_a.x + pos_b.x) * 0.5,
                (pos_a.y + pos_b.y) * 0.5,
                (pos_a.z + pos_b.z) * 0.5,
            );
            let mid_v = self.add_vertex(mid_pos);
            edge_mids.insert((va, vb), mid_v);
        }

        // Capture face data, then delete all affected faces
        struct FaceCaptured {
            verts: Vec<VertexHandle>,
        }
        let mut face_data: Vec<FaceCaptured> = Vec::new();
        for &fh in &affected_faces {
            let verts = self.face_vertices(fh);
            face_data.push(FaceCaptured { verts });
        }
        for &fh in &affected_faces {
            self.delete_face(fh, false);
        }

        // Rebuild each face with midpoints inserted
        let mut new_faces: Vec<FaceHandle> = Vec::new();

        for captured in &face_data {
            let verts = &captured.verts;
            let n = verts.len();

            // Build expanded vertex list with midpoints inserted
            let mut expanded: Vec<VertexHandle> = Vec::new();
            let mut ring_mid_positions: Vec<(usize, VertexHandle)> = Vec::new();

            for i in 0..n {
                expanded.push(verts[i]);
                let key = edge_key(verts[i], verts[(i + 1) % n]);
                if let Some(&mid) = edge_mids.get(&key) {
                    let pos = expanded.len();
                    expanded.push(mid);
                    ring_mid_positions.push((pos, mid));
                }
            }

            let num_ring_edges = ring_mid_positions.len();

            if num_ring_edges == 0 {
                // Face has no ring edges — re-add as-is
                if let Ok(new_fh) = self.add_face(verts) {
                    new_faces.push(new_fh);
                }
            } else if num_ring_edges == 2 && n == 4 {
                // Common case: quad with 2 ring edges -> split into 2 quads
                let mid_a_pos = ring_mid_positions[0].0;
                let mid_b_pos = ring_mid_positions[1].0;
                let exp_len = expanded.len();

                // Build two face vertex lists by walking from mid_a -> mid_b and back
                let mut face1_verts = Vec::new();
                let mut face2_verts = Vec::new();

                let mut idx = mid_a_pos;
                face1_verts.push(expanded[idx]);
                idx = (idx + 1) % exp_len;
                while idx != mid_b_pos {
                    face1_verts.push(expanded[idx]);
                    idx = (idx + 1) % exp_len;
                }
                face1_verts.push(expanded[mid_b_pos]);

                idx = mid_b_pos;
                face2_verts.push(expanded[idx]);
                idx = (idx + 1) % exp_len;
                while idx != mid_a_pos {
                    face2_verts.push(expanded[idx]);
                    idx = (idx + 1) % exp_len;
                }
                face2_verts.push(expanded[mid_a_pos]);

                for sub_verts in [&face1_verts, &face2_verts] {
                    if sub_verts.len() >= 3 {
                        if let Ok(new_fh) = self.add_face(sub_verts) {
                            new_faces.push(new_fh);
                        }
                    }
                }
            } else {
                // General case: re-add expanded polygon as-is
                if expanded.len() >= 3 {
                    if let Ok(new_fh) = self.add_face(&expanded) {
                        new_faces.push(new_fh);
                    }
                }
            }
        }

        // Collect new edges between midpoint vertices
        let mut new_edges: Vec<EdgeHandle> = Vec::new();
        let mid_verts: Vec<VertexHandle> = edge_mids.values().copied().collect();
        for i in 0..mid_verts.len() {
            for j in (i + 1)..mid_verts.len() {
                if let Some(hh) = self.find_halfedge(mid_verts[i], mid_verts[j]) {
                    new_edges.push(hh.edge());
                }
            }
        }

        Ok(LoopCutResult { new_edges, new_faces })
    }

    /// Split an edge at its midpoint, updating adjacent faces.
    ///
    /// The edge is replaced by two edges with a new vertex at the midpoint.
    /// Adjacent faces are rebuilt with the new vertex inserted.
    pub fn split_edge(&mut self, eh: EdgeHandle) -> Result<SplitEdgeResult, MeshError> {
        let h0 = eh.h0();
        let h1 = eh.h1();
        let v0 = self.from_vertex_handle(h0);
        let v1 = self.to_vertex_handle(h0);

        let p0 = self.point(v0).clone();
        let p1 = self.point(v1).clone();
        let mid = nalgebra::Point3::new(
            (p0.x + p1.x) * 0.5,
            (p0.y + p1.y) * 0.5,
            (p0.z + p1.z) * 0.5,
        );
        let vm = self.add_vertex(mid);

        // Collect affected faces and their rebuilt vertex lists
        let mut face_rebuilds: Vec<(FaceHandle, Vec<VertexHandle>)> = Vec::new();

        for &hh in &[h0, h1] {
            if let Some(fh) = self.face_handle(hh) {
                let verts = self.face_vertices(fh);
                let n = verts.len();
                let mut new_verts = Vec::with_capacity(n + 1);

                for i in 0..n {
                    new_verts.push(verts[i]);
                    let next = verts[(i + 1) % n];
                    // Insert vm between consecutive v0->v1 or v1->v0
                    if (verts[i] == v0 && next == v1) || (verts[i] == v1 && next == v0) {
                        new_verts.push(vm);
                    }
                }

                face_rebuilds.push((fh, new_verts));
            }
        }

        // Delete affected faces
        for &(fh, _) in &face_rebuilds {
            self.delete_face(fh, false);
        }

        // Re-add faces with new vertex lists
        let mut new_faces = Vec::new();
        for (_, verts) in &face_rebuilds {
            let new_fh = self.add_face(verts)?;
            new_faces.push(new_fh);
        }

        // Find new edges vm-v0 and vm-v1
        let mut new_edges = Vec::new();
        if let Some(hh) = self.find_halfedge(vm, v0) {
            new_edges.push(hh.edge());
        }
        if let Some(hh) = self.find_halfedge(vm, v1) {
            new_edges.push(hh.edge());
        }

        Ok(SplitEdgeResult {
            new_vertex: vm,
            new_edges,
            new_faces,
        })
    }

    /// Extrude selected edges: duplicate edge vertices, create quad faces
    /// between old and new edges.
    ///
    /// After extrusion, the new vertices sit at the same position as the originals.
    /// The caller should translate `new_vertices` by the desired offset.
    pub fn extrude_edges(
        &mut self,
        edges: &[EdgeHandle],
    ) -> Result<ExtrudeEdgesResult, MeshError> {
        if edges.is_empty() {
            return Err(MeshError::DegenerateFace);
        }

        // Collect unique vertices from all edges
        let mut unique_verts: HashSet<VertexHandle> = HashSet::new();
        for &eh in edges {
            let h0 = eh.h0();
            unique_verts.insert(self.from_vertex_handle(h0));
            unique_verts.insert(self.to_vertex_handle(h0));
        }

        // Duplicate each unique vertex
        let mut old_to_new: HashMap<VertexHandle, VertexHandle> = HashMap::new();
        let mut new_vertices: Vec<VertexHandle> = Vec::new();
        for &v in &unique_verts {
            let pos = *self.point(v);
            let new_v = self.add_vertex(pos);
            old_to_new.insert(v, new_v);
            new_vertices.push(new_v);
        }

        // Create quad faces for each edge
        let mut new_faces: Vec<FaceHandle> = Vec::new();
        for &eh in edges {
            let h0 = eh.h0();
            let h1 = eh.h1();

            let f0 = self.face_handle(h0);
            let f1 = self.face_handle(h1);

            // The new face must attach to the boundary halfedge.
            // If h1 is boundary, the new face uses h1's direction (v1->v0).
            // If h0 is boundary, the new face uses h0's direction (v0->v1 boundary side).
            // For interior edges, use h0's direction.
            let (src, dst) = if f1.is_none() {
                // h1 is boundary, new face attaches via h1 (from=v1, to=v0)
                (self.from_vertex_handle(h1), self.to_vertex_handle(h1))
            } else if f0.is_none() {
                // h0 is boundary, new face attaches via h0
                (self.from_vertex_handle(h0), self.to_vertex_handle(h0))
            } else {
                // Interior edge, use h0's direction
                (self.from_vertex_handle(h0), self.to_vertex_handle(h0))
            };

            let quad = [src, dst, old_to_new[&dst], old_to_new[&src]];
            let new_fh = self.add_face(&quad)?;
            new_faces.push(new_fh);
        }

        Ok(ExtrudeEdgesResult {
            new_vertices,
            new_faces,
        })
    }

    /// Flip the normal of a face by reversing its vertex winding order.
    ///
    /// The face is deleted and re-added with reversed vertices, which flips
    /// the direction of the face normal. Returns the new face handle.
    ///
    /// This only succeeds when every edge of the face is either boundary or
    /// shared with another face that is also being flipped.  To flip multiple
    /// adjacent faces use [`flip_face_normals`] instead.
    pub fn flip_face_normal(
        &mut self,
        fh: FaceHandle,
    ) -> Result<FaceHandle, MeshError> {
        let result = self.flip_face_normals(&[fh])?;
        Ok(result[0])
    }

    /// Flip normals of multiple faces atomically by reversing their winding.
    ///
    /// All faces are deleted and re-added together, so adjacent faces in the
    /// set don't block each other.  Fails if any edge is shared with a face
    /// that is *not* in the set (would create inconsistent orientation).
    pub fn flip_face_normals(
        &mut self,
        faces: &[FaceHandle],
    ) -> Result<Vec<FaceHandle>, MeshError> {
        let face_set: HashSet<FaceHandle> = faces.iter().copied().collect();

        // Collect original and reversed vertex lists, and validate
        let mut original_list: Vec<Vec<VertexHandle>> = Vec::with_capacity(faces.len());
        let mut reversed_list: Vec<Vec<VertexHandle>> = Vec::with_capacity(faces.len());
        for &fh in faces {
            let verts = self.face_vertices(fh);
            if verts.len() < 3 {
                return Err(MeshError::DegenerateFace);
            }

            let mut reversed = verts.clone();
            reversed.reverse();

            // Validate: the reversed halfedge va->vb must not be owned by a
            // face outside the flip set.
            let n = reversed.len();
            for i in 0..n {
                let va = reversed[i];
                let vb = reversed[(i + 1) % n];
                if let Some(hh) = self.find_halfedge(va, vb) {
                    if let Some(adj_fh) = self.face_handle(hh) {
                        if !face_set.contains(&adj_fh) {
                            return Err(MeshError::ComplexEdge);
                        }
                    }
                }
            }

            original_list.push(verts);
            reversed_list.push(reversed);
        }

        // Delete all faces first
        for &fh in faces {
            self.delete_face(fh, false);
        }

        // Re-add with reversed winding; on failure, restore originals
        let mut new_faces = Vec::with_capacity(faces.len());
        for verts in &reversed_list {
            match self.add_face(verts) {
                Ok(new_fh) => new_faces.push(new_fh),
                Err(e) => {
                    // Roll back: delete any already-added reversed faces
                    for &added_fh in &new_faces {
                        self.delete_face(added_fh, false);
                    }
                    // Restore all original faces
                    for orig_verts in &original_list {
                        let _ = self.add_face(orig_verts);
                    }
                    return Err(e);
                }
            }
        }

        Ok(new_faces)
    }
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::Point3;

    /// Build a simple unit cube PolyMesh with 8 vertices and 6 quad faces.
    fn make_cube() -> PolyMesh {
        let mut mesh = PolyMesh::new();

        // 8 vertices of a unit cube
        let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));
        let v4 = mesh.add_vertex(Point3::new(0.0, 0.0, 1.0));
        let v5 = mesh.add_vertex(Point3::new(1.0, 0.0, 1.0));
        let v6 = mesh.add_vertex(Point3::new(1.0, 1.0, 1.0));
        let v7 = mesh.add_vertex(Point3::new(0.0, 1.0, 1.0));

        // 6 faces (CCW winding when viewed from outside)
        mesh.add_face(&[v0, v1, v2, v3]).unwrap(); // front (z=0)
        mesh.add_face(&[v5, v4, v7, v6]).unwrap(); // back (z=1)
        mesh.add_face(&[v4, v0, v3, v7]).unwrap(); // left (x=0)
        mesh.add_face(&[v1, v5, v6, v2]).unwrap(); // right (x=1)
        mesh.add_face(&[v4, v5, v1, v0]).unwrap(); // bottom (y=0)
        mesh.add_face(&[v3, v2, v6, v7]).unwrap(); // top (y=1)

        mesh
    }

    #[test]
    fn test_find_edge_ring_on_cube() {
        let mesh = make_cube();
        // Find an edge and get its ring
        // Edge between v0 and v1 (front face, bottom edge)
        let hh = mesh.find_halfedge(
            VertexHandle::new(0),
            VertexHandle::new(1),
        ).expect("edge should exist");
        let ring = mesh.find_edge_ring(hh.edge());
        // On a cube, an edge ring through one edge of a quad should
        // traverse through 2 faces and produce 2 edges (the start edge
        // plus the opposite edge on the adjacent face).
        assert!(ring.len() >= 2, "edge ring should have at least 2 edges, got {}", ring.len());
    }

    #[test]
    fn test_insert_edge_on_quad() {
        let mut mesh = make_cube();
        let faces: Vec<FaceHandle> = mesh.faces().collect();
        let first_face = faces[0];
        let verts = mesh.face_vertices(first_face);
        assert_eq!(verts.len(), 4);

        // Insert edge between non-adjacent vertices (diagonal)
        let result = mesh.insert_edge(first_face, verts[0], verts[2]);
        assert!(result.is_ok(), "insert_edge should succeed on a quad");

        let result = result.unwrap();
        // After splitting a quad, we should have one more live face than before
        // Original: 6, delete 1 + add 2 = 7 live faces (8 total with the deleted one)
        let live_faces: usize = mesh.faces()
            .filter(|&fh| !mesh.is_deleted_face(fh))
            .count();
        assert_eq!(live_faces, 7);

        // Both result faces should be triangles (splitting a quad diagonally)
        let va_verts = mesh.face_vertices(result.face_a);
        let vb_verts = mesh.face_vertices(result.face_b);
        assert_eq!(va_verts.len(), 3);
        assert_eq!(vb_verts.len(), 3);
    }

    #[test]
    fn test_extrude_single_face() {
        let mut mesh = make_cube();
        let faces: Vec<FaceHandle> = mesh.faces().collect();
        let face_to_extrude = faces[0];

        let initial_verts = mesh.n_vertices();
        let initial_faces = mesh.n_faces();

        let result = mesh.extrude_faces(&[face_to_extrude]);
        assert!(result.is_ok(), "extrude should succeed");

        let result = result.unwrap();
        // Extruding a quad: 4 new vertices
        assert_eq!(result.new_vertices.len(), 4);
        // 1 top face
        assert_eq!(result.top_faces.len(), 1);
        // New vertex count: original + 4 duplicates
        // (Some original verts may be removed if isolated, but on a cube they won't be)
        assert!(mesh.n_vertices() > initial_verts);
        // Face count: original - 1 (deleted) + 1 (top) + 4 (side quads)
        // But after garbage collection the count should reflect the net change
        assert!(mesh.n_faces() > initial_faces);
    }

    #[test]
    fn test_split_edge() {
        let mut mesh = make_cube();
        let _initial_faces = mesh.n_faces();

        let hh = mesh.find_halfedge(VertexHandle::new(0), VertexHandle::new(1))
            .expect("edge should exist");
        let result = mesh.split_edge(hh.edge());
        assert!(result.is_ok(), "split_edge should succeed");

        let result = result.unwrap();
        assert_eq!(result.new_edges.len(), 2, "should have 2 new edges");
        // The midpoint vertex should exist
        let mid_pos = mesh.point(result.new_vertex);
        assert!((mid_pos.x - 0.5).abs() < 1e-10);
        assert!(mid_pos.y.abs() < 1e-10);
        assert!(mid_pos.z.abs() < 1e-10);
    }

    #[test]
    fn test_extrude_edges() {
        // Use a single quad plane so edges are boundary
        let mut mesh = PolyMesh::new();
        let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));
        mesh.add_face(&[v0, v1, v2, v3]).unwrap();

        let initial_verts = mesh.n_vertices();

        // Pick a boundary edge
        let hh = mesh.find_halfedge(v0, v1)
            .expect("edge should exist");

        let result = mesh.extrude_edges(&[hh.edge()]);
        assert!(result.is_ok(), "extrude_edges should succeed");

        let result = result.unwrap();
        assert_eq!(result.new_vertices.len(), 2, "should have 2 new vertices");
        assert_eq!(result.new_faces.len(), 1, "should have 1 new quad face");
        assert_eq!(mesh.n_vertices(), initial_verts + 2);
    }

    #[test]
    fn test_flip_face_normal() {
        // Use a single open quad so flipping doesn't conflict with neighbors
        let mut mesh = PolyMesh::new();
        let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));
        let fh = mesh.add_face(&[v0, v1, v2, v3]).unwrap();

        let original_verts = mesh.face_vertices(fh);

        // Flip the normal
        let new_fh = mesh.flip_face_normal(fh).expect("flip should succeed");
        let flipped_verts = mesh.face_vertices(new_fh);

        // Flipped should be the reverse of original
        let mut expected = original_verts.clone();
        expected.reverse();
        assert_eq!(flipped_verts, expected);

        // Flip again should restore original winding
        let restored_fh = mesh.flip_face_normal(new_fh).expect("flip should succeed");
        let restored_verts = mesh.face_vertices(restored_fh);
        assert_eq!(restored_verts, original_verts);
    }

    #[test]
    fn test_flip_face_normal_closed_mesh_fails() {
        let mut mesh = make_cube();
        let fh = FaceHandle::new(0);
        let initial_faces = mesh.n_faces();

        // Flipping a face on a closed cube should fail — neighbors block the reversed winding
        let result = mesh.flip_face_normal(fh);
        assert!(result.is_err(), "flip on closed mesh should return error");

        // Mesh should be unchanged
        assert_eq!(mesh.n_faces(), initial_faces);
    }

    #[test]
    fn test_flip_face_normals_open_plane() {
        // Build a 2-quad open plane:  v0--v1--v4
        //                             |  f0  | f1 |
        //                             v3--v2--v5
        let mut mesh = PolyMesh::new();
        let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = mesh.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = mesh.add_vertex(Point3::new(0.0, 1.0, 0.0));
        let v4 = mesh.add_vertex(Point3::new(2.0, 0.0, 0.0));
        let v5 = mesh.add_vertex(Point3::new(2.0, 1.0, 0.0));
        let f0 = mesh.add_face(&[v0, v1, v2, v3]).unwrap();
        let f1 = mesh.add_face(&[v1, v4, v5, v2]).unwrap();

        // Flipping only one should fail (shared edge with the other)
        assert!(mesh.flip_face_normal(f0).is_err());

        // Flipping both atomically should succeed
        let new_faces = mesh.flip_face_normals(&[f0, f1]).expect("batch flip should succeed");
        assert_eq!(new_faces.len(), 2);
        let live_faces: usize = mesh.faces().filter(|&fh| !mesh.is_deleted_face(fh)).count();
        assert_eq!(live_faces, 2);
    }

    #[test]
    fn test_flip_face_normals_closed_cube_all() {
        // Flipping ALL faces of a closed cube should succeed
        let mut mesh = make_cube();
        let all_faces: Vec<FaceHandle> = mesh.faces().collect();
        let result = mesh.flip_face_normals(&all_faces);
        assert!(result.is_ok(), "flipping all faces of a closed mesh should succeed");
        assert_eq!(result.unwrap().len(), 6);
        let live_faces: usize = mesh.faces().filter(|&fh| !mesh.is_deleted_face(fh)).count();
        assert_eq!(live_faces, 6);
    }

    #[test]
    fn test_loop_cut_cube() {
        let mut mesh = make_cube();
        let initial_faces = mesh.n_faces();

        // Find an edge to start the loop cut
        let hh = mesh.find_halfedge(
            VertexHandle::new(0),
            VertexHandle::new(1),
        ).expect("edge should exist");

        let result = mesh.loop_cut(hh.edge());
        assert!(result.is_ok(), "loop_cut should succeed");

        let result = result.unwrap();
        // Should have created new edges and faces
        assert!(!result.new_edges.is_empty(), "should have new edges");
        assert!(!result.new_faces.is_empty(), "should have new faces");
        // Loop cut on a cube should increase face count
        assert!(mesh.n_faces() > initial_faces,
            "face count should increase: {} > {}", mesh.n_faces(), initial_faces);
    }
}
