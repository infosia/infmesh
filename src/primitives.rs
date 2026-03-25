use std::collections::HashMap;
use std::f64::consts::PI;

use nalgebra::Point3;

use crate::error::MeshError;
use crate::handle::{FaceHandle, VertexHandle};
use crate::polymesh::PolyMesh;
use crate::trimesh::TriMesh;

/// Trait for generating a primitive mesh along with associated topology data.
pub trait Primitive {
    type Mesh;
    type Data;
    fn generate(self) -> Result<(Self::Mesh, Self::Data), MeshError>;
}

// ---------------------------------------------------------------------------
// Quad
// ---------------------------------------------------------------------------

pub struct Quad;

pub struct QuadData {
    pub face: FaceHandle,
}

impl Primitive for Quad {
    type Mesh = PolyMesh;
    type Data = QuadData;

    fn generate(self) -> Result<(PolyMesh, QuadData), MeshError> {
        let mut mesh = PolyMesh::new();
        let v0 = mesh.add_vertex(Point3::new(-0.5, 0.0, 0.5));
        let v1 = mesh.add_vertex(Point3::new(0.5, 0.0, 0.5));
        let v2 = mesh.add_vertex(Point3::new(0.5, 0.0, -0.5));
        let v3 = mesh.add_vertex(Point3::new(-0.5, 0.0, -0.5));
        let face = mesh.add_face(&[v0, v1, v2, v3])?;
        Ok((mesh, QuadData { face }))
    }
}

// ---------------------------------------------------------------------------
// Circle
// ---------------------------------------------------------------------------

pub struct Circle {
    pub segments: usize,
}

pub struct CircleData {
    pub face: FaceHandle,
}

impl Primitive for Circle {
    type Mesh = PolyMesh;
    type Data = CircleData;

    fn generate(self) -> Result<(PolyMesh, CircleData), MeshError> {
        if self.segments < 3 {
            return Err(MeshError::DegenerateFace);
        }

        let mut mesh = PolyMesh::new();
        let radius = 0.5;
        let angle_increment = 2.0 * PI / self.segments as f64;

        let mut verts = Vec::with_capacity(self.segments);
        for i in 0..self.segments {
            let angle = i as f64 * angle_increment;
            let x = radius * angle.cos();
            let z = -radius * angle.sin();
            verts.push(mesh.add_vertex(Point3::new(x, 0.0, z)));
        }

        let face = mesh.add_face(&verts)?;
        Ok((mesh, CircleData { face }))
    }
}

// ---------------------------------------------------------------------------
// Cube
// ---------------------------------------------------------------------------

pub struct Cube {
    pub subdivisions: [u16; 3],
}

pub struct CubeData {
    pub front_bottom_left_vertex: VertexHandle,
}

impl Primitive for Cube {
    type Mesh = PolyMesh;
    type Data = CubeData;

    fn generate(self) -> Result<(PolyMesh, CubeData), MeshError> {
        let [n_x, n_y, n_z] = self.subdivisions;
        let dx = 1.0 / n_x as f64;
        let dy = 1.0 / n_y as f64;
        let dz = 1.0 / n_z as f64;

        let mut mesh = PolyMesh::new();
        let mut vertex_map: HashMap<(u16, u16, u16), VertexHandle> = HashMap::new();

        // Generate vertices on the cube surface
        for x in 0..=n_x {
            for y in 0..=n_y {
                for z in 0..=n_z {
                    if x == 0 || x == n_x || y == 0 || y == n_y || z == 0 || z == n_z {
                        let pos = Point3::new(
                            -0.5 + x as f64 * dx,
                            -0.5 + y as f64 * dy,
                            -0.5 + z as f64 * dz,
                        );
                        let vh = mesh.add_vertex(pos);
                        vertex_map.insert((x, y, z), vh);
                    }
                }
            }
        }

        let v = |x, y, z| *vertex_map.get(&(x, y, z)).unwrap();

        // Front face (z = n_z)
        for x in 0..n_x {
            for y in 0..n_y {
                mesh.add_face(&[v(x, y, n_z), v(x + 1, y, n_z), v(x + 1, y + 1, n_z), v(x, y + 1, n_z)])?;
            }
        }
        // Back face (z = 0)
        for x in 0..n_x {
            for y in 0..n_y {
                mesh.add_face(&[v(x, y, 0), v(x, y + 1, 0), v(x + 1, y + 1, 0), v(x + 1, y, 0)])?;
            }
        }
        // Left face (x = 0)
        for y in 0..n_y {
            for z in 0..n_z {
                mesh.add_face(&[v(0, y, z), v(0, y, z + 1), v(0, y + 1, z + 1), v(0, y + 1, z)])?;
            }
        }
        // Right face (x = n_x)
        for y in 0..n_y {
            for z in 0..n_z {
                mesh.add_face(&[v(n_x, y, z), v(n_x, y + 1, z), v(n_x, y + 1, z + 1), v(n_x, y, z + 1)])?;
            }
        }
        // Top face (y = n_y)
        for x in 0..n_x {
            for z in 0..n_z {
                mesh.add_face(&[v(x, n_y, z), v(x, n_y, z + 1), v(x + 1, n_y, z + 1), v(x + 1, n_y, z)])?;
            }
        }
        // Bottom face (y = 0)
        for x in 0..n_x {
            for z in 0..n_z {
                mesh.add_face(&[v(x, 0, z), v(x + 1, 0, z), v(x + 1, 0, z + 1), v(x, 0, z + 1)])?;
            }
        }

        Ok((mesh, CubeData { front_bottom_left_vertex: v(0, 0, n_z) }))
    }
}

// ---------------------------------------------------------------------------
// Icosphere
// ---------------------------------------------------------------------------

pub struct Icosphere {
    pub subdivisions: usize,
}

pub struct IcosphereData {
    pub top_vertex: VertexHandle,
    pub bottom_vertex: VertexHandle,
}

impl Primitive for Icosphere {
    type Mesh = TriMesh;
    type Data = IcosphereData;

    fn generate(self) -> Result<(TriMesh, IcosphereData), MeshError> {
        let mut mesh = TriMesh::new();
        let t = (1.0 + 5.0_f64.sqrt()) / 2.0;

        let base_verts = [
            [-1.0, t, 0.0],
            [1.0, t, 0.0],
            [-1.0, -t, 0.0],
            [1.0, -t, 0.0],
            [0.0, -1.0, t],
            [0.0, 1.0, t],
            [0.0, -1.0, -t],
            [0.0, 1.0, -t],
            [t, 0.0, -1.0],
            [t, 0.0, 1.0],
            [-t, 0.0, -1.0],
            [-t, 0.0, 1.0],
        ];

        let mut vertex_ids: Vec<VertexHandle> = Vec::new();
        for v in &base_verts {
            let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            let pos = Point3::new(v[0] / len * 0.5, v[1] / len * 0.5, v[2] / len * 0.5);
            vertex_ids.push(mesh.add_vertex(pos));
        }

        let ico_faces: [[usize; 3]; 20] = [
            [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
            [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
            [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
            [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
        ];

        let mut tri_indices: Vec<[VertexHandle; 3]> = ico_faces
            .iter()
            .map(|f| [vertex_ids[f[0]], vertex_ids[f[1]], vertex_ids[f[2]]])
            .collect();

        // Subdivide
        let mut midpoint_cache = HashMap::<(VertexHandle, VertexHandle), VertexHandle>::new();

        for _ in 0..self.subdivisions {
            let mut new_tris = Vec::new();
            for tri in &tri_indices {
                let a = get_midpoint(&mut mesh, &mut midpoint_cache, tri[0], tri[1]);
                let b = get_midpoint(&mut mesh, &mut midpoint_cache, tri[1], tri[2]);
                let c = get_midpoint(&mut mesh, &mut midpoint_cache, tri[2], tri[0]);

                new_tris.push([tri[0], a, c]);
                new_tris.push([tri[1], b, a]);
                new_tris.push([tri[2], c, b]);
                new_tris.push([a, b, c]);
            }
            tri_indices = new_tris;
        }

        // Add all faces
        for tri in &tri_indices {
            mesh.add_face(&[tri[0], tri[1], tri[2]])?;
        }

        // Normalize all vertices to lie on the sphere
        let n_verts = mesh.n_vertices();
        for i in 0..n_verts {
            let vh = VertexHandle::new(i as u32);
            let p = mesh.point(vh).clone();
            let len = (p.x * p.x + p.y * p.y + p.z * p.z).sqrt();
            if len > 0.0 {
                *mesh.point_mut(vh) = Point3::new(p.x / len * 0.5, p.y / len * 0.5, p.z / len * 0.5);
            }
        }

        Ok((
            mesh,
            IcosphereData {
                top_vertex: vertex_ids[0],
                bottom_vertex: vertex_ids[3],
            },
        ))
    }
}

fn get_midpoint(
    mesh: &mut TriMesh,
    cache: &mut HashMap<(VertexHandle, VertexHandle), VertexHandle>,
    v1: VertexHandle,
    v2: VertexHandle,
) -> VertexHandle {
    let key = if v1 < v2 { (v1, v2) } else { (v2, v1) };
    if let Some(&mid) = cache.get(&key) {
        return mid;
    }
    let p1 = mesh.point(v1).clone();
    let p2 = mesh.point(v2).clone();
    let mid_pos = Point3::new(
        (p1.x + p2.x) * 0.5,
        (p1.y + p2.y) * 0.5,
        (p1.z + p2.z) * 0.5,
    );
    // Normalize to sphere surface
    let len = (mid_pos.x * mid_pos.x + mid_pos.y * mid_pos.y + mid_pos.z * mid_pos.z).sqrt();
    let normalized = Point3::new(mid_pos.x / len * 0.5, mid_pos.y / len * 0.5, mid_pos.z / len * 0.5);
    let vh = mesh.add_vertex(normalized);
    cache.insert(key, vh);
    vh
}

// ---------------------------------------------------------------------------
// UvSphere
// ---------------------------------------------------------------------------

pub struct UvSphere {
    pub segments: usize,
    pub rings: usize,
}

pub struct UvSphereData {
    pub top_vertex: VertexHandle,
    pub bottom_vertex: VertexHandle,
}

impl Primitive for UvSphere {
    type Mesh = PolyMesh;
    type Data = UvSphereData;

    fn generate(self) -> Result<(PolyMesh, UvSphereData), MeshError> {
        if self.segments < 3 || self.rings < 2 {
            return Err(MeshError::DegenerateFace);
        }

        let mut mesh = PolyMesh::new();
        let radius = 0.5;

        // Top pole
        let top = mesh.add_vertex(Point3::new(0.0, radius, 0.0));

        // Ring vertices (rings-1 internal rings between the two poles)
        let mut ring_verts: Vec<Vec<VertexHandle>> = Vec::new();
        for ring in 1..self.rings {
            let phi = PI * ring as f64 / self.rings as f64;
            let y = radius * phi.cos();
            let ring_radius = radius * phi.sin();

            let mut row = Vec::with_capacity(self.segments);
            for seg in 0..self.segments {
                let theta = 2.0 * PI * seg as f64 / self.segments as f64;
                let x = ring_radius * theta.cos();
                let z = ring_radius * theta.sin();
                row.push(mesh.add_vertex(Point3::new(x, y, z)));
            }
            ring_verts.push(row);
        }

        // Bottom pole
        let bottom = mesh.add_vertex(Point3::new(0.0, -radius, 0.0));

        // Top cap (triangle fan)
        for seg in 0..self.segments {
            let next = (seg + 1) % self.segments;
            mesh.add_face(&[top, ring_verts[0][next], ring_verts[0][seg]])?;
        }

        // Middle quads
        for ring in 0..ring_verts.len() - 1 {
            for seg in 0..self.segments {
                let next = (seg + 1) % self.segments;
                mesh.add_face(&[
                    ring_verts[ring][seg],
                    ring_verts[ring][next],
                    ring_verts[ring + 1][next],
                    ring_verts[ring + 1][seg],
                ])?;
            }
        }

        // Bottom cap (triangle fan)
        let last_ring = ring_verts.len() - 1;
        for seg in 0..self.segments {
            let next = (seg + 1) % self.segments;
            mesh.add_face(&[ring_verts[last_ring][seg], ring_verts[last_ring][next], bottom])?;
        }

        Ok((mesh, UvSphereData { top_vertex: top, bottom_vertex: bottom }))
    }
}

// ---------------------------------------------------------------------------
// Cylinder
// ---------------------------------------------------------------------------

pub struct Cylinder {
    pub segments: usize,
    pub height: f64,
    pub radius: f64,
}

pub struct CylinderData {
    pub top_face: FaceHandle,
    pub bottom_face: FaceHandle,
}

impl Primitive for Cylinder {
    type Mesh = PolyMesh;
    type Data = CylinderData;

    fn generate(self) -> Result<(PolyMesh, CylinderData), MeshError> {
        if self.segments < 3 {
            return Err(MeshError::DegenerateFace);
        }

        let mut mesh = PolyMesh::new();
        let mut top_verts = Vec::new();
        let mut bottom_verts = Vec::new();

        for i in 0..self.segments {
            let angle = (i as f64 / self.segments as f64) * 2.0 * PI;
            let x = angle.cos() * self.radius;
            let z = angle.sin() * self.radius;

            bottom_verts.push(mesh.add_vertex(Point3::new(x, -self.height / 2.0, z)));
            top_verts.push(mesh.add_vertex(Point3::new(x, self.height / 2.0, z)));
        }

        // Side faces
        for i in 0..self.segments {
            let next = (i + 1) % self.segments;
            mesh.add_face(&[bottom_verts[i], top_verts[i], top_verts[next], bottom_verts[next]])?;
        }

        // Cap faces
        let bottom_face = mesh.add_face(&bottom_verts)?;
        let top_verts_rev: Vec<_> = top_verts.iter().rev().copied().collect();
        let top_face = mesh.add_face(&top_verts_rev)?;

        Ok((mesh, CylinderData { top_face, bottom_face }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quad() {
        let (mesh, data) = Quad.generate().unwrap();
        assert_eq!(mesh.n_vertices(), 4);
        assert_eq!(mesh.n_faces(), 1);
        assert_eq!(data.face, FaceHandle::new(0));
    }

    #[test]
    fn test_circle() {
        let (mesh, data) = Circle { segments: 6 }.generate().unwrap();
        assert_eq!(mesh.n_vertices(), 6);
        assert_eq!(mesh.n_faces(), 1);
        assert_eq!(data.face, FaceHandle::new(0));
    }

    #[test]
    fn test_circle_too_few_segments() {
        let result = Circle { segments: 2 }.generate();
        assert!(result.is_err());
    }

    #[test]
    fn test_cube_unit() {
        let (mesh, _data) = Cube { subdivisions: [1, 1, 1] }.generate().unwrap();
        assert_eq!(mesh.n_vertices(), 8);
        assert_eq!(mesh.n_faces(), 6);
    }

    #[test]
    fn test_cube_subdivided() {
        let (mesh, _data) = Cube { subdivisions: [2, 2, 2] }.generate().unwrap();
        // 2x2x2 cube: each face has 2x2 = 4 quads, 6 faces = 24 quads
        assert_eq!(mesh.n_faces(), 24);
    }

    #[test]
    fn test_icosphere_base() {
        let (mesh, _data) = Icosphere { subdivisions: 0 }.generate().unwrap();
        assert_eq!(mesh.n_vertices(), 12);
        assert_eq!(mesh.n_faces(), 20);
    }

    #[test]
    fn test_icosphere_subdivided() {
        let (mesh, _data) = Icosphere { subdivisions: 1 }.generate().unwrap();
        // 1 subdivision: 20 * 4 = 80 faces, 12 + 30 = 42 vertices
        assert_eq!(mesh.n_faces(), 80);
        assert_eq!(mesh.n_vertices(), 42);
    }

    #[test]
    fn test_cylinder() {
        let (mesh, _data) = Cylinder {
            segments: 8,
            height: 1.0,
            radius: 0.5,
        }
        .generate()
        .unwrap();
        // 8 side quads + 2 cap faces = 10
        assert_eq!(mesh.n_faces(), 10);
        assert_eq!(mesh.n_vertices(), 16);
    }

    #[test]
    fn test_uv_sphere() {
        let (mesh, data) = UvSphere { segments: 8, rings: 4 }.generate().unwrap();
        // 2 poles + 3 internal rings * 8 segments = 2 + 24 = 26 vertices
        assert_eq!(mesh.n_vertices(), 26);
        // 8 top tris + 2*8 middle quads + 8 bottom tris = 32 faces
        assert_eq!(mesh.n_faces(), 32);
        assert_eq!(data.top_vertex, VertexHandle::new(0));
        assert_eq!(data.bottom_vertex, VertexHandle::new(25));
    }

    #[test]
    fn test_uv_sphere_minimum() {
        let (mesh, _) = UvSphere { segments: 3, rings: 2 }.generate().unwrap();
        // 2 poles + 1 ring * 3 = 5 vertices
        assert_eq!(mesh.n_vertices(), 5);
        // 3 top tris + 3 bottom tris = 6 faces
        assert_eq!(mesh.n_faces(), 6);
    }

    #[test]
    fn test_uv_sphere_too_few() {
        assert!(UvSphere { segments: 2, rings: 2 }.generate().is_err());
        assert!(UvSphere { segments: 3, rings: 1 }.generate().is_err());
    }

    #[test]
    fn test_cylinder_too_few_segments() {
        let result = Cylinder { segments: 2, height: 1.0, radius: 0.5 }.generate();
        assert!(result.is_err());
    }
}
