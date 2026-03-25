use crate::handle::{EdgeHandle, FaceHandle, HalfedgeHandle, VertexHandle};

macro_rules! define_iterator {
    ($name:ident, $handle:ty) => {
        #[derive(Clone, Debug)]
        pub struct $name {
            current: u32,
            end: u32,
        }

        impl $name {
            #[inline]
            pub(crate) fn new(start: u32, end: u32) -> Self {
                Self { current: start, end }
            }
        }

        impl Iterator for $name {
            type Item = $handle;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                if self.current < self.end {
                    let h = <$handle>::new(self.current);
                    self.current += 1;
                    Some(h)
                } else {
                    None
                }
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                let remaining = (self.end - self.current) as usize;
                (remaining, Some(remaining))
            }
        }

        impl ExactSizeIterator for $name {}

        impl DoubleEndedIterator for $name {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                if self.current < self.end {
                    self.end -= 1;
                    Some(<$handle>::new(self.end))
                } else {
                    None
                }
            }
        }
    };
}

define_iterator!(VertexIter, VertexHandle);
define_iterator!(HalfedgeIter, HalfedgeHandle);
define_iterator!(EdgeIter, EdgeHandle);
define_iterator!(FaceIter, FaceHandle);

use super::connectivity::Connectivity;

impl Connectivity {
    /// Iterate over all vertex handles.
    #[inline]
    pub fn vertices(&self) -> VertexIter {
        VertexIter::new(0, self.n_vertices() as u32)
    }

    /// Iterate over all halfedge handles.
    #[inline]
    pub fn halfedges(&self) -> HalfedgeIter {
        HalfedgeIter::new(0, self.n_halfedges() as u32)
    }

    /// Iterate over all edge handles.
    #[inline]
    pub fn edges(&self) -> EdgeIter {
        EdgeIter::new(0, self.n_edges() as u32)
    }

    /// Iterate over all face handles.
    #[inline]
    pub fn faces(&self) -> FaceIter {
        FaceIter::new(0, self.n_faces() as u32)
    }
}
