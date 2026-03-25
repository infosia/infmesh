use crate::handle::{EdgeHandle, FaceHandle, HalfedgeHandle, VertexHandle};
use super::connectivity::Connectivity;

// ============================================================================
// Vertex-centered circulators
//
// These iterate around a vertex by walking the outgoing halfedge ring.
// CW rotation: next(opposite(heh))
// CCW rotation: opposite(prev(heh))
// ============================================================================

/// Generic vertex circulator that walks the outgoing halfedge ring.
#[derive(Clone, Debug)]
pub struct VertexCirculator<'a> {
    conn: &'a Connectivity,
    start: HalfedgeHandle,
    current: HalfedgeHandle,
    active: bool,
    cw: bool,
}

impl<'a> VertexCirculator<'a> {
    fn new(conn: &'a Connectivity, vh: VertexHandle, cw: bool) -> Option<Self> {
        let hh = conn.vertex_halfedge_handle(vh)?;
        Some(Self {
            conn,
            start: hh,
            current: hh,
            active: false,
            cw,
        })
    }

    #[inline]
    fn advance(&mut self) {
        if self.cw {
            self.current = self.conn.cw_rotated_halfedge_handle(self.current);
        } else {
            self.current = self.conn.ccw_rotated_halfedge_handle(self.current);
        }
    }
}

// --- VertexVertex ---

pub struct VVIterator<'a>(Option<VertexCirculator<'a>>);

impl<'a> Iterator for VVIterator<'a> {
    type Item = VertexHandle;

    fn next(&mut self) -> Option<Self::Item> {
        let circ = self.0.as_mut()?;
        if circ.active && circ.current == circ.start {
            return None;
        }
        circ.active = true;
        let vh = circ.conn.to_vertex_handle(circ.current);
        circ.advance();
        Some(vh)
    }
}

// --- VertexOutgoingHalfedge ---

pub struct VOHIterator<'a>(Option<VertexCirculator<'a>>);

impl<'a> Iterator for VOHIterator<'a> {
    type Item = HalfedgeHandle;

    fn next(&mut self) -> Option<Self::Item> {
        let circ = self.0.as_mut()?;
        if circ.active && circ.current == circ.start {
            return None;
        }
        circ.active = true;
        let hh = circ.current;
        circ.advance();
        Some(hh)
    }
}

// --- VertexIncomingHalfedge ---

pub struct VIHIterator<'a>(Option<VertexCirculator<'a>>);

impl<'a> Iterator for VIHIterator<'a> {
    type Item = HalfedgeHandle;

    fn next(&mut self) -> Option<Self::Item> {
        let circ = self.0.as_mut()?;
        if circ.active && circ.current == circ.start {
            return None;
        }
        circ.active = true;
        let hh = circ.current.opposite();
        circ.advance();
        Some(hh)
    }
}

// --- VertexEdge ---

pub struct VEIterator<'a>(Option<VertexCirculator<'a>>);

impl<'a> Iterator for VEIterator<'a> {
    type Item = EdgeHandle;

    fn next(&mut self) -> Option<Self::Item> {
        let circ = self.0.as_mut()?;
        if circ.active && circ.current == circ.start {
            return None;
        }
        circ.active = true;
        let eh = circ.current.edge();
        circ.advance();
        Some(eh)
    }
}

// --- VertexFace (skips boundary halfedges) ---

pub struct VFIterator<'a>(Option<VertexCirculator<'a>>);

impl<'a> Iterator for VFIterator<'a> {
    type Item = FaceHandle;

    fn next(&mut self) -> Option<Self::Item> {
        let circ = self.0.as_mut()?;
        loop {
            if circ.active && circ.current == circ.start {
                return None;
            }
            circ.active = true;
            let face = circ.conn.face_handle(circ.current);
            circ.advance();
            if let Some(fh) = face {
                return Some(fh);
            }
        }
    }
}

// ============================================================================
// Face-centered circulators
//
// These iterate around a face by walking the halfedge loop.
// CCW (default): next_halfedge_handle
// CW: prev_halfedge_handle
// ============================================================================

#[derive(Clone, Debug)]
pub struct FaceCirculator<'a> {
    conn: &'a Connectivity,
    start: HalfedgeHandle,
    current: HalfedgeHandle,
    active: bool,
    cw: bool,
}

impl<'a> FaceCirculator<'a> {
    fn new(conn: &'a Connectivity, fh: FaceHandle, cw: bool) -> Self {
        let hh = conn.face_halfedge_handle(fh);
        Self {
            conn,
            start: hh,
            current: hh,
            active: false,
            cw,
        }
    }

    #[inline]
    fn advance(&mut self) {
        if self.cw {
            self.current = self.conn.prev_halfedge_handle(self.current);
        } else {
            self.current = self.conn.next_halfedge_handle(self.current);
        }
    }
}

// --- FaceVertex ---

pub struct FVIterator<'a>(FaceCirculator<'a>);

impl<'a> Iterator for FVIterator<'a> {
    type Item = VertexHandle;

    fn next(&mut self) -> Option<Self::Item> {
        let circ = &mut self.0;
        if circ.active && circ.current == circ.start {
            return None;
        }
        circ.active = true;
        let vh = circ.conn.to_vertex_handle(circ.current);
        circ.advance();
        Some(vh)
    }
}

// --- FaceHalfedge ---

pub struct FHIterator<'a>(FaceCirculator<'a>);

impl<'a> Iterator for FHIterator<'a> {
    type Item = HalfedgeHandle;

    fn next(&mut self) -> Option<Self::Item> {
        let circ = &mut self.0;
        if circ.active && circ.current == circ.start {
            return None;
        }
        circ.active = true;
        let hh = circ.current;
        circ.advance();
        Some(hh)
    }
}

// --- FaceEdge ---

pub struct FEIterator<'a>(FaceCirculator<'a>);

impl<'a> Iterator for FEIterator<'a> {
    type Item = EdgeHandle;

    fn next(&mut self) -> Option<Self::Item> {
        let circ = &mut self.0;
        if circ.active && circ.current == circ.start {
            return None;
        }
        circ.active = true;
        let eh = circ.current.edge();
        circ.advance();
        Some(eh)
    }
}

// --- FaceFace (skips boundary, yields adjacent face handles) ---

pub struct FFIterator<'a>(FaceCirculator<'a>);

impl<'a> Iterator for FFIterator<'a> {
    type Item = FaceHandle;

    fn next(&mut self) -> Option<Self::Item> {
        let circ = &mut self.0;
        loop {
            if circ.active && circ.current == circ.start {
                return None;
            }
            circ.active = true;
            let face = circ.conn.face_handle(circ.current.opposite());
            circ.advance();
            if let Some(fh) = face {
                return Some(fh);
            }
        }
    }
}

// --- HalfedgeLoop (same as FaceHalfedge but started from a halfedge) ---

pub struct HLIterator<'a> {
    conn: &'a Connectivity,
    start: HalfedgeHandle,
    current: HalfedgeHandle,
    active: bool,
    cw: bool,
}

impl<'a> Iterator for HLIterator<'a> {
    type Item = HalfedgeHandle;

    fn next(&mut self) -> Option<Self::Item> {
        if self.active && self.current == self.start {
            return None;
        }
        self.active = true;
        let hh = self.current;
        if self.cw {
            self.current = self.conn.prev_halfedge_handle(self.current);
        } else {
            self.current = self.conn.next_halfedge_handle(self.current);
        }
        Some(hh)
    }
}

// ============================================================================
// Convenience methods on Connectivity
// ============================================================================

impl Connectivity {
    // --- Vertex-centered ---

    pub fn vv_cw_iter(&self, vh: VertexHandle) -> VVIterator<'_> {
        VVIterator(VertexCirculator::new(self, vh, true))
    }

    pub fn vv_ccw_iter(&self, vh: VertexHandle) -> VVIterator<'_> {
        VVIterator(VertexCirculator::new(self, vh, false))
    }

    pub fn voh_cw_iter(&self, vh: VertexHandle) -> VOHIterator<'_> {
        VOHIterator(VertexCirculator::new(self, vh, true))
    }

    pub fn voh_ccw_iter(&self, vh: VertexHandle) -> VOHIterator<'_> {
        VOHIterator(VertexCirculator::new(self, vh, false))
    }

    pub fn vih_cw_iter(&self, vh: VertexHandle) -> VIHIterator<'_> {
        VIHIterator(VertexCirculator::new(self, vh, true))
    }

    pub fn vih_ccw_iter(&self, vh: VertexHandle) -> VIHIterator<'_> {
        VIHIterator(VertexCirculator::new(self, vh, false))
    }

    pub fn ve_cw_iter(&self, vh: VertexHandle) -> VEIterator<'_> {
        VEIterator(VertexCirculator::new(self, vh, true))
    }

    pub fn ve_ccw_iter(&self, vh: VertexHandle) -> VEIterator<'_> {
        VEIterator(VertexCirculator::new(self, vh, false))
    }

    pub fn vf_cw_iter(&self, vh: VertexHandle) -> VFIterator<'_> {
        VFIterator(VertexCirculator::new(self, vh, true))
    }

    pub fn vf_ccw_iter(&self, vh: VertexHandle) -> VFIterator<'_> {
        VFIterator(VertexCirculator::new(self, vh, false))
    }

    // --- Face-centered ---

    pub fn fv_ccw_iter(&self, fh: FaceHandle) -> FVIterator<'_> {
        FVIterator(FaceCirculator::new(self, fh, false))
    }

    pub fn fv_cw_iter(&self, fh: FaceHandle) -> FVIterator<'_> {
        FVIterator(FaceCirculator::new(self, fh, true))
    }

    pub fn fh_ccw_iter(&self, fh: FaceHandle) -> FHIterator<'_> {
        FHIterator(FaceCirculator::new(self, fh, false))
    }

    pub fn fh_cw_iter(&self, fh: FaceHandle) -> FHIterator<'_> {
        FHIterator(FaceCirculator::new(self, fh, true))
    }

    pub fn fe_ccw_iter(&self, fh: FaceHandle) -> FEIterator<'_> {
        FEIterator(FaceCirculator::new(self, fh, false))
    }

    pub fn fe_cw_iter(&self, fh: FaceHandle) -> FEIterator<'_> {
        FEIterator(FaceCirculator::new(self, fh, true))
    }

    pub fn ff_ccw_iter(&self, fh: FaceHandle) -> FFIterator<'_> {
        FFIterator(FaceCirculator::new(self, fh, false))
    }

    pub fn ff_cw_iter(&self, fh: FaceHandle) -> FFIterator<'_> {
        FFIterator(FaceCirculator::new(self, fh, true))
    }

    // --- Halfedge loop ---

    pub fn hl_ccw_iter(&self, hh: HalfedgeHandle) -> HLIterator<'_> {
        HLIterator {
            conn: self,
            start: hh,
            current: hh,
            active: false,
            cw: false,
        }
    }

    pub fn hl_cw_iter(&self, hh: HalfedgeHandle) -> HLIterator<'_> {
        HLIterator {
            conn: self,
            start: hh,
            current: hh,
            active: false,
            cw: true,
        }
    }
}
