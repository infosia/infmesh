pub mod handle;
pub mod error;
pub mod mesh;
pub mod property;
pub mod trimesh;
pub mod polymesh;
pub mod geometry;
pub mod io;
pub mod algo;
pub mod primitives;

/// Scalar type used for all geometry (vertex positions, normals, etc.).
/// Change this to `f64` if double precision is needed.
pub type Scalar = f32;

pub use handle::{EdgeHandle, FaceHandle, HalfedgeHandle, Status, VertexHandle};
pub use error::MeshError;
pub use mesh::Connectivity;
pub use property::{PropertyStore, PropertyHandle, VPropHandle, HPropHandle, EPropHandle, FPropHandle, PropertyKind, HandleKind};
pub use trimesh::TriMesh;
pub use polymesh::PolyMesh;
pub use primitives::{Primitive, Quad, Circle, Cube, Cylinder, Icosphere, UvSphere};
