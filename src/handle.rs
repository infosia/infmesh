use std::fmt;

use bitflags::bitflags;

macro_rules! define_handle {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u32);

        impl $name {
            #[inline]
            pub fn new(idx: u32) -> Self {
                Self(idx)
            }

            #[inline]
            pub fn idx(self) -> usize {
                self.0 as usize
            }

            #[inline]
            pub fn idx_u32(self) -> u32 {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<u32> for $name {
            #[inline]
            fn from(idx: u32) -> Self {
                Self(idx)
            }
        }

        impl From<usize> for $name {
            #[inline]
            fn from(idx: usize) -> Self {
                Self(idx as u32)
            }
        }

        impl From<$name> for usize {
            #[inline]
            fn from(h: $name) -> usize {
                h.0 as usize
            }
        }

        impl From<$name> for u32 {
            #[inline]
            fn from(h: $name) -> u32 {
                h.0
            }
        }
    };
}

define_handle!(VertexHandle, "Handle for a vertex entity.");
define_handle!(HalfedgeHandle, "Handle for a halfedge entity.");
define_handle!(EdgeHandle, "Handle for an edge entity.");
define_handle!(FaceHandle, "Handle for a face entity.");

// --- Edge <-> Halfedge conversions ---

impl EdgeHandle {
    /// Get the i-th halfedge (0 or 1) of this edge.
    #[inline]
    pub fn halfedge(self, i: u32) -> HalfedgeHandle {
        debug_assert!(i <= 1);
        HalfedgeHandle(self.0 * 2 + i)
    }

    /// Shorthand for `self.halfedge(0)`.
    #[inline]
    pub fn h0(self) -> HalfedgeHandle {
        self.halfedge(0)
    }

    /// Shorthand for `self.halfedge(1)`.
    #[inline]
    pub fn h1(self) -> HalfedgeHandle {
        self.halfedge(1)
    }
}

impl HalfedgeHandle {
    /// Get the edge this halfedge belongs to.
    #[inline]
    pub fn edge(self) -> EdgeHandle {
        EdgeHandle(self.0 / 2)
    }

    /// Get the opposite halfedge (the other halfedge of the same edge).
    #[inline]
    pub fn opposite(self) -> HalfedgeHandle {
        HalfedgeHandle(self.0 ^ 1)
    }
}

// --- Status Flags ---

bitflags! {
    /// Status bits for mesh elements (vertices, edges, halfedges, faces).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Status: u32 {
        const DELETED            = 1;
        const LOCKED             = 2;
        const SELECTED           = 4;
        const HIDDEN             = 8;
        const FEATURE            = 16;
        const TAGGED             = 32;
        const TAGGED2            = 64;
        const FIXED_NONMANIFOLD  = 128;
    }
}

impl Status {
    #[inline]
    pub fn is_deleted(self) -> bool {
        self.contains(Status::DELETED)
    }

    #[inline]
    pub fn is_locked(self) -> bool {
        self.contains(Status::LOCKED)
    }

    #[inline]
    pub fn is_selected(self) -> bool {
        self.contains(Status::SELECTED)
    }

    #[inline]
    pub fn is_hidden(self) -> bool {
        self.contains(Status::HIDDEN)
    }

    #[inline]
    pub fn is_feature(self) -> bool {
        self.contains(Status::FEATURE)
    }

    #[inline]
    pub fn is_tagged(self) -> bool {
        self.contains(Status::TAGGED)
    }

    #[inline]
    pub fn is_tagged2(self) -> bool {
        self.contains(Status::TAGGED2)
    }

    #[inline]
    pub fn set_deleted(&mut self, val: bool) {
        self.set(Status::DELETED, val);
    }

    #[inline]
    pub fn set_locked(&mut self, val: bool) {
        self.set(Status::LOCKED, val);
    }

    #[inline]
    pub fn set_selected(&mut self, val: bool) {
        self.set(Status::SELECTED, val);
    }

    #[inline]
    pub fn set_hidden(&mut self, val: bool) {
        self.set(Status::HIDDEN, val);
    }

    #[inline]
    pub fn set_feature(&mut self, val: bool) {
        self.set(Status::FEATURE, val);
    }

    #[inline]
    pub fn set_tagged(&mut self, val: bool) {
        self.set(Status::TAGGED, val);
    }

    #[inline]
    pub fn set_tagged2(&mut self, val: bool) {
        self.set(Status::TAGGED2, val);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_creation_and_idx() {
        let vh = VertexHandle::new(42);
        assert_eq!(vh.idx(), 42);
        assert_eq!(vh.idx_u32(), 42);

        let eh = EdgeHandle::new(10);
        assert_eq!(eh.idx(), 10);

        let fh = FaceHandle::new(0);
        assert_eq!(fh.idx(), 0);
    }

    #[test]
    fn handle_equality_and_ordering() {
        let a = VertexHandle::new(1);
        let b = VertexHandle::new(1);
        let c = VertexHandle::new(2);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a < c);
    }

    #[test]
    fn handle_from_conversions() {
        let vh = VertexHandle::from(5u32);
        assert_eq!(vh.idx(), 5);

        let vh2 = VertexHandle::from(7usize);
        assert_eq!(vh2.idx(), 7);

        let idx: usize = vh.into();
        assert_eq!(idx, 5);

        let idx32: u32 = vh.into();
        assert_eq!(idx32, 5);
    }

    #[test]
    fn edge_halfedge_conversion() {
        let eh = EdgeHandle::new(3);
        let h0 = eh.halfedge(0);
        let h1 = eh.halfedge(1);

        assert_eq!(h0.idx(), 6);
        assert_eq!(h1.idx(), 7);
        assert_eq!(eh.h0(), h0);
        assert_eq!(eh.h1(), h1);

        // halfedge -> edge
        assert_eq!(h0.edge(), eh);
        assert_eq!(h1.edge(), eh);
    }

    #[test]
    fn halfedge_opposite() {
        let h0 = HalfedgeHandle::new(6);
        let h1 = h0.opposite();
        assert_eq!(h1.idx(), 7);
        assert_eq!(h1.opposite(), h0);

        let h2 = HalfedgeHandle::new(0);
        assert_eq!(h2.opposite().idx(), 1);
        assert_eq!(h2.opposite().opposite(), h2);

        let h3 = HalfedgeHandle::new(5);
        assert_eq!(h3.opposite().idx(), 4);
    }

    #[test]
    fn opposite_preserves_edge() {
        for i in 0..20u32 {
            let hh = HalfedgeHandle::new(i);
            assert_eq!(hh.edge(), hh.opposite().edge());
        }
    }

    #[test]
    fn handle_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(VertexHandle::new(1));
        set.insert(VertexHandle::new(2));
        set.insert(VertexHandle::new(1));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn handle_debug_display() {
        let vh = VertexHandle::new(42);
        assert_eq!(format!("{:?}", vh), "VertexHandle(42)");
        assert_eq!(format!("{}", vh), "42");
    }

    // --- Status tests ---

    #[test]
    fn status_default_empty() {
        let s = Status::default();
        assert!(!s.is_deleted());
        assert!(!s.is_locked());
        assert!(!s.is_selected());
        assert!(!s.is_hidden());
        assert!(!s.is_feature());
        assert!(!s.is_tagged());
        assert!(!s.is_tagged2());
    }

    #[test]
    fn status_set_and_query() {
        let mut s = Status::default();

        s.set_deleted(true);
        assert!(s.is_deleted());
        assert!(!s.is_locked());

        s.set_locked(true);
        assert!(s.is_deleted());
        assert!(s.is_locked());

        s.set_deleted(false);
        assert!(!s.is_deleted());
        assert!(s.is_locked());
    }

    #[test]
    fn status_bitwise() {
        let s = Status::DELETED | Status::FEATURE;
        assert!(s.is_deleted());
        assert!(s.is_feature());
        assert!(!s.is_tagged());
    }

    #[test]
    fn status_values_match_cpp() {
        // Verify bit values match C++ OpenMesh StatusBits
        assert_eq!(Status::DELETED.bits(), 1);
        assert_eq!(Status::LOCKED.bits(), 2);
        assert_eq!(Status::SELECTED.bits(), 4);
        assert_eq!(Status::HIDDEN.bits(), 8);
        assert_eq!(Status::FEATURE.bits(), 16);
        assert_eq!(Status::TAGGED.bits(), 32);
        assert_eq!(Status::TAGGED2.bits(), 64);
        assert_eq!(Status::FIXED_NONMANIFOLD.bits(), 128);
    }
}
