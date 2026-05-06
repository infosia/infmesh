use crate::handle::{EdgeHandle, FaceHandle, HalfedgeHandle, Status, VertexHandle};
use crate::error::MeshError;

// --- Internal data types ---

#[derive(Clone, Debug, Default)]
pub(crate) struct VertexData {
    pub halfedge: Option<HalfedgeHandle>,
}

#[derive(Clone, Debug)]
pub(crate) struct HalfedgeData {
    pub face: Option<FaceHandle>,
    pub vertex: VertexHandle, // target vertex
    pub next: HalfedgeHandle,
    pub prev: HalfedgeHandle,
}

impl Default for HalfedgeData {
    fn default() -> Self {
        Self {
            face: None,
            vertex: VertexHandle::new(0),
            next: HalfedgeHandle::new(0),
            prev: HalfedgeHandle::new(0),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct EdgeData {
    pub halfedges: [HalfedgeData; 2],
}

#[derive(Clone, Debug)]
pub(crate) struct FaceData {
    pub halfedge: HalfedgeHandle,
}

impl Default for FaceData {
    fn default() -> Self {
        Self {
            halfedge: HalfedgeHandle::new(0),
        }
    }
}

// --- Scratch data for add_face ---

#[derive(Clone, Debug, Default)]
struct EdgeInfo {
    halfedge_handle: Option<HalfedgeHandle>,
    is_new: bool,
    needs_adjust: bool,
}

// --- Connectivity ---

#[derive(Clone, Debug, Default)]
pub struct Connectivity {
    pub(crate) vertices: Vec<VertexData>,
    pub(crate) edges: Vec<EdgeData>,
    pub(crate) faces: Vec<FaceData>,
    // Scratch space for add_face (avoids repeated allocation)
    edge_data: Vec<EdgeInfo>,
    next_cache: Vec<(HalfedgeHandle, HalfedgeHandle)>,
    // Status
    pub(crate) vertex_status: Vec<Status>,
    pub(crate) edge_status: Vec<Status>,
    pub(crate) halfedge_status: Vec<Status>,
    pub(crate) face_status: Vec<Status>,
}

impl Connectivity {
    pub fn new() -> Self {
        Self::default()
    }

    // === Counts ===

    #[inline]
    pub fn n_vertices(&self) -> usize {
        self.vertices.len()
    }

    #[inline]
    pub fn n_edges(&self) -> usize {
        self.edges.len()
    }

    #[inline]
    pub fn n_halfedges(&self) -> usize {
        self.edges.len() * 2
    }

    #[inline]
    pub fn n_faces(&self) -> usize {
        self.faces.len()
    }

    // === Low-level creation ===

    /// Add a new isolated vertex. Returns its handle.
    pub fn new_vertex(&mut self) -> VertexHandle {
        let idx = self.vertices.len();
        self.vertices.push(VertexData::default());
        self.vertex_status.push(Status::default());
        VertexHandle::new(idx as u32)
    }

    /// Create a new edge between two vertices. Returns the halfedge handle
    /// pointing from `start` to `end` (halfedge 0 of the new edge).
    pub fn new_edge(&mut self, start: VertexHandle, end: VertexHandle) -> HalfedgeHandle {
        let edge_idx = self.edges.len();
        let mut edge = EdgeData::default();
        // halfedge 0 points to `end`, halfedge 1 points to `start`
        edge.halfedges[0].vertex = end;
        edge.halfedges[1].vertex = start;
        self.edges.push(edge);
        self.edge_status.push(Status::default());
        self.halfedge_status.push(Status::default());
        self.halfedge_status.push(Status::default());
        EdgeHandle::new(edge_idx as u32).h0()
    }

    /// Create a new face. Returns its handle.
    pub fn new_face(&mut self) -> FaceHandle {
        let idx = self.faces.len();
        self.faces.push(FaceData::default());
        self.face_status.push(Status::default());
        FaceHandle::new(idx as u32)
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.edges.clear();
        self.faces.clear();
        self.vertex_status.clear();
        self.edge_status.clear();
        self.halfedge_status.clear();
        self.face_status.clear();
    }

    // === Halfedge access ===

    #[inline]
    fn he(&self, hh: HalfedgeHandle) -> &HalfedgeData {
        &self.edges[hh.idx() >> 1].halfedges[hh.idx() & 1]
    }

    #[inline]
    fn he_mut(&mut self, hh: HalfedgeHandle) -> &mut HalfedgeData {
        &mut self.edges[hh.idx() >> 1].halfedges[hh.idx() & 1]
    }

    // === Topology queries ===

    /// Target vertex of a halfedge.
    #[inline]
    pub fn to_vertex_handle(&self, hh: HalfedgeHandle) -> VertexHandle {
        self.he(hh).vertex
    }

    /// Source vertex of a halfedge.
    #[inline]
    pub fn from_vertex_handle(&self, hh: HalfedgeHandle) -> VertexHandle {
        self.to_vertex_handle(hh.opposite())
    }

    /// Face of a halfedge (`None` if boundary).
    #[inline]
    pub fn face_handle(&self, hh: HalfedgeHandle) -> Option<FaceHandle> {
        self.he(hh).face
    }

    /// Next halfedge in face/boundary loop.
    #[inline]
    pub fn next_halfedge_handle(&self, hh: HalfedgeHandle) -> HalfedgeHandle {
        self.he(hh).next
    }

    /// Previous halfedge in face/boundary loop.
    #[inline]
    pub fn prev_halfedge_handle(&self, hh: HalfedgeHandle) -> HalfedgeHandle {
        self.he(hh).prev
    }

    /// Rotate halfedge counter-clockwise around its target vertex.
    /// Returns `opposite(prev(hh))`.
    #[inline]
    pub fn ccw_rotated_halfedge_handle(&self, hh: HalfedgeHandle) -> HalfedgeHandle {
        self.prev_halfedge_handle(hh).opposite()
    }

    /// Rotate halfedge clockwise around its target vertex.
    /// Returns `next(opposite(hh))`.
    #[inline]
    pub fn cw_rotated_halfedge_handle(&self, hh: HalfedgeHandle) -> HalfedgeHandle {
        self.next_halfedge_handle(hh.opposite())
    }

    /// One outgoing halfedge of a vertex (`None` if isolated).
    #[inline]
    pub fn vertex_halfedge_handle(&self, vh: VertexHandle) -> Option<HalfedgeHandle> {
        self.vertices[vh.idx()].halfedge
    }

    /// One halfedge of a face.
    #[inline]
    pub fn face_halfedge_handle(&self, fh: FaceHandle) -> HalfedgeHandle {
        self.faces[fh.idx()].halfedge
    }

    /// Get the i-th halfedge of an edge (0 or 1).
    #[inline]
    pub fn edge_halfedge_handle(&self, eh: EdgeHandle, i: u32) -> HalfedgeHandle {
        eh.halfedge(i)
    }

    // === Setters ===

    #[inline]
    pub fn set_vertex_handle(&mut self, hh: HalfedgeHandle, vh: VertexHandle) {
        self.he_mut(hh).vertex = vh;
    }

    #[inline]
    pub fn set_face_handle(&mut self, hh: HalfedgeHandle, fh: Option<FaceHandle>) {
        self.he_mut(hh).face = fh;
    }

    /// Set next halfedge. Also updates prev of the next halfedge.
    #[inline]
    pub fn set_next_halfedge_handle(&mut self, hh: HalfedgeHandle, nhh: HalfedgeHandle) {
        self.he_mut(hh).next = nhh;
        self.he_mut(nhh).prev = hh;
    }

    #[inline]
    pub fn set_vertex_halfedge_handle(&mut self, vh: VertexHandle, hh: Option<HalfedgeHandle>) {
        self.vertices[vh.idx()].halfedge = hh;
    }

    #[inline]
    pub fn set_face_halfedge_handle(&mut self, fh: FaceHandle, hh: HalfedgeHandle) {
        self.faces[fh.idx()].halfedge = hh;
    }

    // === Boundary queries ===

    /// A halfedge is boundary if it has no face.
    #[inline]
    pub fn is_boundary_halfedge(&self, hh: HalfedgeHandle) -> bool {
        self.he(hh).face.is_none()
    }

    /// A vertex is boundary if it is isolated or its outgoing halfedge is boundary.
    /// More precisely, a vertex is boundary if any of its outgoing halfedges is boundary.
    pub fn is_boundary_vertex(&self, vh: VertexHandle) -> bool {
        let Some(hh) = self.vertex_halfedge_handle(vh) else {
            return true; // isolated vertices are considered boundary
        };
        // The vertex stores a preferred outgoing halfedge. If it's boundary, done.
        if self.is_boundary_halfedge(hh) {
            return true;
        }
        // Otherwise walk around
        let mut curr = self.cw_rotated_halfedge_handle(hh);
        while curr != hh {
            if self.is_boundary_halfedge(curr) {
                return true;
            }
            curr = self.cw_rotated_halfedge_handle(curr);
        }
        false
    }

    /// An edge is boundary if either of its halfedges is boundary.
    #[inline]
    pub fn is_boundary_edge(&self, eh: EdgeHandle) -> bool {
        self.is_boundary_halfedge(eh.h0()) || self.is_boundary_halfedge(eh.h1())
    }

    /// A face is boundary if any of its halfedges' opposite halfedge is boundary.
    pub fn is_boundary_face(&self, fh: FaceHandle) -> bool {
        let start = self.face_halfedge_handle(fh);
        let mut hh = start;
        loop {
            if self.is_boundary_halfedge(hh.opposite()) {
                return true;
            }
            hh = self.next_halfedge_handle(hh);
            if hh == start {
                break;
            }
        }
        false
    }

    /// A vertex is isolated if it has no outgoing halfedge.
    #[inline]
    pub fn is_isolated(&self, vh: VertexHandle) -> bool {
        self.vertices[vh.idx()].halfedge.is_none()
    }

    /// Check if vertex is manifold (all faces around it form a single ring).
    pub fn is_manifold(&self, vh: VertexHandle) -> bool {
        let Some(hh) = self.vertex_halfedge_handle(vh) else {
            return true;
        };
        // Count boundary halfedges around the vertex
        let mut boundary_count = 0;
        let mut curr = hh;
        loop {
            if self.is_boundary_halfedge(curr) {
                boundary_count += 1;
            }
            curr = self.cw_rotated_halfedge_handle(curr);
            if curr == hh {
                break;
            }
        }
        // A manifold vertex has 0 (interior) or 1 (boundary) boundary halfedges in the ring
        // Actually, the outgoing halfedge ring counts 0 boundary sectors for interior
        // or exactly 1 boundary gap for boundary vertices.
        boundary_count <= 1
    }

    /// Vertex valence (number of incident edges).
    pub fn valence_vertex(&self, vh: VertexHandle) -> usize {
        let Some(hh) = self.vertex_halfedge_handle(vh) else {
            return 0;
        };
        let mut count = 0usize;
        let mut curr = hh;
        loop {
            count += 1;
            curr = self.cw_rotated_halfedge_handle(curr);
            if curr == hh {
                break;
            }
        }
        count
    }

    /// Face valence (number of vertices/edges of the face).
    pub fn valence_face(&self, fh: FaceHandle) -> usize {
        let start = self.face_halfedge_handle(fh);
        let mut count = 0usize;
        let mut hh = start;
        loop {
            count += 1;
            hh = self.next_halfedge_handle(hh);
            if hh == start {
                break;
            }
        }
        count
    }

    // === Halfedge search ===

    /// Find the halfedge from `start` to `end`. Returns `None` if not found.
    pub fn find_halfedge(&self, start: VertexHandle, end: VertexHandle) -> Option<HalfedgeHandle> {
        let hh = self.vertex_halfedge_handle(start)?;
        let mut curr = hh;
        loop {
            if self.to_vertex_handle(curr) == end {
                return Some(curr);
            }
            curr = self.cw_rotated_halfedge_handle(curr);
            if curr == hh {
                break;
            }
        }
        None
    }

    /// Adjust the outgoing halfedge of a vertex to prefer a boundary halfedge.
    pub fn adjust_outgoing_halfedge(&mut self, vh: VertexHandle) {
        let Some(hh) = self.vertex_halfedge_handle(vh) else {
            return;
        };
        let mut curr = hh;
        loop {
            if self.is_boundary_halfedge(curr) {
                self.set_vertex_halfedge_handle(vh, Some(curr));
                return;
            }
            curr = self.cw_rotated_halfedge_handle(curr);
            if curr == hh {
                break;
            }
        }
    }

    // === Status access ===

    #[inline]
    pub fn vertex_status(&self, vh: VertexHandle) -> Status {
        self.vertex_status[vh.idx()]
    }

    #[inline]
    pub fn vertex_status_mut(&mut self, vh: VertexHandle) -> &mut Status {
        &mut self.vertex_status[vh.idx()]
    }

    #[inline]
    pub fn edge_status(&self, eh: EdgeHandle) -> Status {
        self.edge_status[eh.idx()]
    }

    #[inline]
    pub fn edge_status_mut(&mut self, eh: EdgeHandle) -> &mut Status {
        &mut self.edge_status[eh.idx()]
    }

    #[inline]
    pub fn halfedge_status(&self, hh: HalfedgeHandle) -> Status {
        self.halfedge_status[hh.idx()]
    }

    #[inline]
    pub fn halfedge_status_mut(&mut self, hh: HalfedgeHandle) -> &mut Status {
        &mut self.halfedge_status[hh.idx()]
    }

    #[inline]
    pub fn face_status(&self, fh: FaceHandle) -> Status {
        self.face_status[fh.idx()]
    }

    #[inline]
    pub fn face_status_mut(&mut self, fh: FaceHandle) -> &mut Status {
        &mut self.face_status[fh.idx()]
    }

    #[inline]
    pub fn is_deleted_vertex(&self, vh: VertexHandle) -> bool {
        self.vertex_status[vh.idx()].is_deleted()
    }

    #[inline]
    pub fn is_deleted_edge(&self, eh: EdgeHandle) -> bool {
        self.edge_status[eh.idx()].is_deleted()
    }

    #[inline]
    pub fn is_deleted_halfedge(&self, hh: HalfedgeHandle) -> bool {
        self.halfedge_status[hh.idx()].is_deleted()
    }

    #[inline]
    pub fn is_deleted_face(&self, fh: FaceHandle) -> bool {
        self.face_status[fh.idx()].is_deleted()
    }

    // === Deletion ===

    /// Delete a vertex and all its incident faces.
    pub fn delete_vertex(&mut self, vh: VertexHandle, delete_isolated_vertices: bool) {
        // Collect incident faces
        let face_handles: Vec<FaceHandle> = self.vf_cw_iter(vh).collect();

        // Delete collected faces
        for fh in face_handles {
            self.delete_face(fh, delete_isolated_vertices);
        }

        self.vertex_status_mut(vh).set_deleted(true);
    }

    /// Delete an edge and its incident faces.
    pub fn delete_edge(&mut self, eh: EdgeHandle, delete_isolated_vertices: bool) {
        let fh0 = self.face_handle(eh.h0());
        let fh1 = self.face_handle(eh.h1());

        if let Some(fh) = fh0 {
            self.delete_face(fh, delete_isolated_vertices);
        }
        if let Some(fh) = fh1 {
            self.delete_face(fh, delete_isolated_vertices);
        }

        // If both sides were boundary (no face), mark the edge itself
        if fh0.is_none() && fh1.is_none() {
            self.edge_status_mut(eh).set_deleted(true);
            self.halfedge_status_mut(eh.h0()).set_deleted(true);
            self.halfedge_status_mut(eh.h1()).set_deleted(true);
        }
    }

    /// Delete a face, update topology, optionally remove isolated vertices.
    pub fn delete_face(&mut self, fh: FaceHandle, delete_isolated_vertices: bool) {
        assert!(!self.face_status(fh).is_deleted());

        // Mark face deleted
        self.face_status_mut(fh).set_deleted(true);

        // Collect boundary edges and vertices
        let mut deleted_edges: Vec<EdgeHandle> = Vec::with_capacity(3);
        let mut vhandles: Vec<VertexHandle> = Vec::with_capacity(3);

        // For all halfedges of face:
        // 1) set face to None (boundary)
        // 2) collect fully-boundary edges
        // 3) store vertex handles
        let start = self.face_halfedge_handle(fh);
        let mut hh = start;
        loop {
            self.set_face_handle(hh, None); // set_boundary
            if self.is_boundary_halfedge(hh.opposite()) {
                deleted_edges.push(hh.edge());
            }
            vhandles.push(self.to_vertex_handle(hh));
            hh = self.next_halfedge_handle(hh);
            if hh == start {
                break;
            }
        }

        // Delete collected edges (both halfedges are now boundary)
        for &eh in &deleted_edges {
            let h0 = eh.h0();
            let v0 = self.to_vertex_handle(h0);
            let next0 = self.next_halfedge_handle(h0);
            let prev0 = self.prev_halfedge_handle(h0);

            let h1 = eh.h1();
            let v1 = self.to_vertex_handle(h1);
            let next1 = self.next_halfedge_handle(h1);
            let prev1 = self.prev_halfedge_handle(h1);

            // Adjust next and prev handles
            self.set_next_halfedge_handle(prev0, next1);
            self.set_next_halfedge_handle(prev1, next0);

            // Mark edge deleted
            self.edge_status_mut(eh).set_deleted(true);
            self.halfedge_status_mut(h0).set_deleted(true);
            self.halfedge_status_mut(h1).set_deleted(true);

            // Update v0
            if self.vertex_halfedge_handle(v0) == Some(h1) {
                if next0 == h1 {
                    // Isolated
                    if delete_isolated_vertices {
                        self.vertex_status_mut(v0).set_deleted(true);
                    }
                    self.set_vertex_halfedge_handle(v0, None);
                } else {
                    self.set_vertex_halfedge_handle(v0, Some(next0));
                }
            }

            // Update v1
            if self.vertex_halfedge_handle(v1) == Some(h0) {
                if next1 == h0 {
                    // Isolated
                    if delete_isolated_vertices {
                        self.vertex_status_mut(v1).set_deleted(true);
                    }
                    self.set_vertex_halfedge_handle(v1, None);
                } else {
                    self.set_vertex_halfedge_handle(v1, Some(next1));
                }
            }
        }

        // Update outgoing halfedge handles of remaining vertices
        for &vh in &vhandles {
            self.adjust_outgoing_halfedge(vh);
        }
    }

    /// Compact arrays by removing all deleted elements and remapping handles.
    /// Returns the vertex mapping (old index -> new VertexHandle) so callers
    /// can remap external data (e.g. geometry, properties).
    pub fn garbage_collection(&mut self) -> Vec<VertexHandle> {
        let nv = self.n_vertices();
        let ne = self.n_edges();
        let nf = self.n_faces();

        // Build vertex mapping (old -> new)
        let mut v_map: Vec<VertexHandle> = Vec::with_capacity(nv);
        let mut new_vi = 0u32;
        for i in 0..nv {
            if self.vertex_status[i].is_deleted() {
                v_map.push(VertexHandle::new(u32::MAX)); // sentinel
            } else {
                v_map.push(VertexHandle::new(new_vi));
                if new_vi as usize != i {
                    self.vertices[new_vi as usize] = self.vertices[i].clone();
                    self.vertex_status[new_vi as usize] = self.vertex_status[i];
                }
                new_vi += 1;
            }
        }
        self.vertices.truncate(new_vi as usize);
        self.vertex_status.truncate(new_vi as usize);

        // Build edge/halfedge mapping (old -> new)
        let mut he_map: Vec<HalfedgeHandle> = vec![HalfedgeHandle::new(u32::MAX); ne * 2];
        let mut new_ei = 0u32;
        for i in 0..ne {
            if self.edge_status[i].is_deleted() {
                // both halfedges map to sentinel
            } else {
                if new_ei as usize != i {
                    self.edges[new_ei as usize] = self.edges[i].clone();
                    self.edge_status[new_ei as usize] = self.edge_status[i];
                    self.halfedge_status[new_ei as usize * 2] = self.halfedge_status[i * 2];
                    self.halfedge_status[new_ei as usize * 2 + 1] = self.halfedge_status[i * 2 + 1];
                }
                he_map[i * 2] = HalfedgeHandle::new(new_ei * 2);
                he_map[i * 2 + 1] = HalfedgeHandle::new(new_ei * 2 + 1);
                new_ei += 1;
            }
        }
        self.edges.truncate(new_ei as usize);
        self.edge_status.truncate(new_ei as usize);
        self.halfedge_status.truncate(new_ei as usize * 2);

        // Build face mapping (old -> new)
        let mut f_map: Vec<FaceHandle> = Vec::with_capacity(nf);
        let mut new_fi = 0u32;
        for i in 0..nf {
            if self.face_status[i].is_deleted() {
                f_map.push(FaceHandle::new(u32::MAX)); // sentinel
            } else {
                f_map.push(FaceHandle::new(new_fi));
                if new_fi as usize != i {
                    self.faces[new_fi as usize] = self.faces[i].clone();
                    self.face_status[new_fi as usize] = self.face_status[i];
                }
                new_fi += 1;
            }
        }
        self.faces.truncate(new_fi as usize);
        self.face_status.truncate(new_fi as usize);

        // Remap vertex -> halfedge
        for vd in &mut self.vertices {
            if let Some(ref mut hh) = vd.halfedge {
                *hh = he_map[hh.idx()];
            }
        }

        // Remap halfedge fields
        for ei in 0..self.edges.len() {
            for side in 0..2 {
                let hd = &mut self.edges[ei].halfedges[side];
                hd.vertex = v_map[hd.vertex.idx()];
                hd.next = he_map[hd.next.idx()];
                hd.prev = he_map[hd.prev.idx()];
                if let Some(ref mut fh) = hd.face {
                    *fh = f_map[fh.idx()];
                }
            }
        }

        // Remap face -> halfedge
        for fd in &mut self.faces {
            fd.halfedge = he_map[fd.halfedge.idx()];
        }

        v_map
    }

    // === add_face ===

    /// Add a face defined by the given vertex handles.
    ///
    /// Returns the new face handle, or an error if the topology is invalid
    /// (e.g., non-manifold vertex or edge).
    pub fn add_face(&mut self, vertex_handles: &[VertexHandle]) -> Result<FaceHandle, MeshError> {
        let n = vertex_handles.len();
        if n < 3 {
            return Err(MeshError::DegenerateFace);
        }

        // Ensure scratch space
        self.edge_data.clear();
        self.edge_data.resize(n, EdgeInfo::default());
        self.next_cache.clear();

        // Phase 1: Test for topological errors and find existing halfedges
        for i in 0..n {
            let ii = (i + 1) % n;

            if !self.is_boundary_vertex(vertex_handles[i]) {
                return Err(MeshError::ComplexVertex);
            }

            let hh = self.find_halfedge(vertex_handles[i], vertex_handles[ii]);
            self.edge_data[i].halfedge_handle = hh;
            self.edge_data[i].is_new = hh.is_none();
            self.edge_data[i].needs_adjust = false;

            if let Some(hh) = hh
                && !self.is_boundary_halfedge(hh)
            {
                return Err(MeshError::ComplexEdge);
            }
        }

        // Phase 2: Re-link patches if necessary
        for i in 0..n {
            let ii = (i + 1) % n;

            if !self.edge_data[i].is_new && !self.edge_data[ii].is_new {
                let inner_prev = self.edge_data[i].halfedge_handle
                    .expect("existing edge must have halfedge handle set in phase 1");
                let inner_next = self.edge_data[ii].halfedge_handle
                    .expect("existing edge must have halfedge handle set in phase 1");

                if self.next_halfedge_handle(inner_prev) != inner_next {
                    // Need to relink a patch
                    let outer_prev = inner_next.opposite();
                    let mut boundary_prev = outer_prev;
                    loop {
                        boundary_prev = self.next_halfedge_handle(boundary_prev).opposite();
                        if self.is_boundary_halfedge(boundary_prev) {
                            break;
                        }
                    }
                    let boundary_next = self.next_halfedge_handle(boundary_prev);

                    if boundary_prev == inner_prev {
                        return Err(MeshError::PatchRelinkFailed);
                    }

                    let patch_start = self.next_halfedge_handle(inner_prev);
                    let patch_end = self.prev_halfedge_handle(inner_next);

                    self.next_cache.push((boundary_prev, patch_start));
                    self.next_cache.push((patch_end, boundary_next));
                    self.next_cache.push((inner_prev, inner_next));
                }
            }
        }

        // Phase 3: Create missing edges
        for i in 0..n {
            let ii = (i + 1) % n;
            if self.edge_data[i].is_new {
                let hh = self.new_edge(vertex_handles[i], vertex_handles[ii]);
                self.edge_data[i].halfedge_handle = Some(hh);
            }
        }

        // Phase 4: Create the face
        let fh = self.new_face();
        self.set_face_halfedge_handle(fh, self.edge_data[n - 1].halfedge_handle
            .expect("edge must have halfedge handle after phase 3"));

        // Phase 5: Setup halfedges
        for i in 0..n {
            let ii = (i + 1) % n;
            let vh = vertex_handles[ii];

            let inner_prev = self.edge_data[i].halfedge_handle
                .expect("edge must have halfedge handle after phase 3");
            let inner_next = self.edge_data[ii].halfedge_handle
                .expect("edge must have halfedge handle after phase 3");

            let id = (if self.edge_data[i].is_new { 1u32 } else { 0 })
                | (if self.edge_data[ii].is_new { 2 } else { 0 });

            if id != 0 {
                let outer_prev = inner_next.opposite();
                let outer_next = inner_prev.opposite();

                match id {
                    1 => {
                        // prev is new, next is old
                        let boundary_prev = self.prev_halfedge_handle(inner_next);
                        self.next_cache.push((boundary_prev, outer_next));
                        self.set_vertex_halfedge_handle(vh, Some(outer_next));
                    }
                    2 => {
                        // next is new, prev is old
                        let boundary_next = self.next_halfedge_handle(inner_prev);
                        self.next_cache.push((outer_prev, boundary_next));
                        self.set_vertex_halfedge_handle(vh, Some(boundary_next));
                    }
                    3 => {
                        // both are new
                        if self.vertex_halfedge_handle(vh).is_none() {
                            self.set_vertex_halfedge_handle(vh, Some(outer_next));
                            self.next_cache.push((outer_prev, outer_next));
                        } else {
                            let boundary_next = self.vertex_halfedge_handle(vh)
                                .expect("vertex must have outgoing halfedge in case 3 else branch");
                            let boundary_prev = self.prev_halfedge_handle(boundary_next);
                            self.next_cache.push((boundary_prev, outer_next));
                            self.next_cache.push((outer_prev, boundary_next));
                        }
                    }
                    _ => unreachable!(),
                }

                // Set inner link
                self.next_cache.push((inner_prev, inner_next));
            } else {
                // Both are old
                self.edge_data[ii].needs_adjust =
                    self.vertex_halfedge_handle(vh) == Some(inner_next);
            }

            // Set face handle
            self.set_face_handle(inner_prev, Some(fh));
        }

        // Phase 6: Process next halfedge cache
        // Take the cache out to avoid borrow issues
        let cache = std::mem::take(&mut self.next_cache);
        for &(hh, nhh) in &cache {
            self.set_next_halfedge_handle(hh, nhh);
        }
        self.next_cache = cache;

        // Phase 7: Adjust vertices' halfedge handle
        // Note: edge_data[i].needs_adjust was set for vertex_handles[i] in Phase 2,
        // so the i-th flag corresponds to the i-th input vertex.
        for (i, &vh) in vertex_handles.iter().enumerate().take(n) {
            if self.edge_data[i].needs_adjust {
                self.adjust_outgoing_halfedge(vh);
            }
        }

        Ok(fh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_vertex() {
        let mut conn = Connectivity::new();
        let v0 = conn.new_vertex();
        let v1 = conn.new_vertex();
        assert_eq!(v0.idx(), 0);
        assert_eq!(v1.idx(), 1);
        assert_eq!(conn.n_vertices(), 2);
        assert!(conn.is_isolated(v0));
        assert!(conn.is_isolated(v1));
    }

    #[test]
    fn new_edge() {
        let mut conn = Connectivity::new();
        let v0 = conn.new_vertex();
        let v1 = conn.new_vertex();
        let hh = conn.new_edge(v0, v1);

        assert_eq!(conn.n_edges(), 1);
        assert_eq!(conn.n_halfedges(), 2);

        // h0 points to v1, h1 points to v0
        assert_eq!(conn.to_vertex_handle(hh), v1);
        assert_eq!(conn.to_vertex_handle(hh.opposite()), v0);
        assert_eq!(conn.from_vertex_handle(hh), v0);

        // Edge <-> halfedge
        assert_eq!(hh.edge(), EdgeHandle::new(0));
        assert_eq!(hh.opposite().edge(), EdgeHandle::new(0));
    }

    #[test]
    fn add_single_triangle() {
        let mut conn = Connectivity::new();
        let v0 = conn.new_vertex();
        let v1 = conn.new_vertex();
        let v2 = conn.new_vertex();

        let fh = conn.add_face(&[v0, v1, v2]).unwrap();
        assert_eq!(conn.n_faces(), 1);
        assert_eq!(conn.n_edges(), 3);
        assert_eq!(conn.n_halfedges(), 6);

        // Face halfedge loop should have 3 halfedges
        assert_eq!(conn.valence_face(fh), 3);

        // All interior halfedges should point to fh
        let start = conn.face_halfedge_handle(fh);
        let mut hh = start;
        loop {
            assert_eq!(conn.face_handle(hh), Some(fh));
            hh = conn.next_halfedge_handle(hh);
            if hh == start {
                break;
            }
        }

        // All boundary halfedges (opposite) should have no face
        hh = start;
        loop {
            assert!(conn.is_boundary_halfedge(hh.opposite()));
            hh = conn.next_halfedge_handle(hh);
            if hh == start {
                break;
            }
        }
    }

    #[test]
    fn add_two_triangles_sharing_edge() {
        let mut conn = Connectivity::new();
        let v0 = conn.new_vertex();
        let v1 = conn.new_vertex();
        let v2 = conn.new_vertex();
        let v3 = conn.new_vertex();

        // Triangle 1: v2-v1-v0
        conn.add_face(&[v2, v1, v0]).unwrap();
        // Triangle 2: v2-v0-v3
        conn.add_face(&[v2, v0, v3]).unwrap();

        assert_eq!(conn.n_vertices(), 4);
        assert_eq!(conn.n_faces(), 2);
        assert_eq!(conn.n_edges(), 5);
        assert_eq!(conn.n_halfedges(), 10);
    }

    #[test]
    fn add_quad() {
        let mut conn = Connectivity::new();
        let v0 = conn.new_vertex();
        let v1 = conn.new_vertex();
        let v2 = conn.new_vertex();
        let v3 = conn.new_vertex();

        let fh = conn.add_face(&[v0, v1, v2, v3]).unwrap();
        assert_eq!(conn.n_faces(), 1);
        assert_eq!(conn.n_edges(), 4);
        assert_eq!(conn.valence_face(fh), 4);
    }

    #[test]
    fn boundary_queries_single_triangle() {
        let mut conn = Connectivity::new();
        let v0 = conn.new_vertex();
        let v1 = conn.new_vertex();
        let v2 = conn.new_vertex();

        let fh = conn.add_face(&[v0, v1, v2]).unwrap();

        // All vertices of a single triangle are boundary
        assert!(conn.is_boundary_vertex(v0));
        assert!(conn.is_boundary_vertex(v1));
        assert!(conn.is_boundary_vertex(v2));

        // The face is boundary (all edges touch the boundary)
        assert!(conn.is_boundary_face(fh));

        // All edges are boundary
        for i in 0..conn.n_edges() {
            assert!(conn.is_boundary_edge(EdgeHandle::new(i as u32)));
        }
    }

    #[test]
    fn find_halfedge_basic() {
        let mut conn = Connectivity::new();
        let v0 = conn.new_vertex();
        let v1 = conn.new_vertex();
        let v2 = conn.new_vertex();

        conn.add_face(&[v0, v1, v2]).unwrap();

        assert!(conn.find_halfedge(v0, v1).is_some());
        assert!(conn.find_halfedge(v1, v2).is_some());
        assert!(conn.find_halfedge(v2, v0).is_some());
        assert!(conn.find_halfedge(v0, v2).is_some()); // exists as opposite direction
    }

    #[test]
    fn valence() {
        let mut conn = Connectivity::new();
        let v0 = conn.new_vertex();
        let v1 = conn.new_vertex();
        let v2 = conn.new_vertex();
        let v3 = conn.new_vertex();

        conn.add_face(&[v2, v1, v0]).unwrap();
        conn.add_face(&[v2, v0, v3]).unwrap();

        // v0 and v2 are shared by both faces
        assert_eq!(conn.valence_vertex(v0), 3);
        assert_eq!(conn.valence_vertex(v2), 3);
        // v1 and v3 are each in one face only
        assert_eq!(conn.valence_vertex(v1), 2);
        assert_eq!(conn.valence_vertex(v3), 2);
    }
}
