use thiserror::Error;

#[derive(Debug, Error)]
pub enum MeshError {
    #[error("complex vertex: vertex is not on boundary")]
    ComplexVertex,

    #[error("complex edge: edge is not on boundary")]
    ComplexEdge,

    #[error("patch re-linking failed: no free gap found")]
    PatchRelinkFailed,

    #[error("face must have at least 3 vertices")]
    DegenerateFace,

    #[error("cannot flip a boundary edge")]
    BoundaryEdge,

    #[error("invalid property handle")]
    InvalidPropertyHandle,

    #[error("property type mismatch")]
    PropertyTypeMismatch,
}
