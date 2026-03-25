use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use nalgebra::{Point3, Vector3};

use crate::Scalar;
use crate::handle::VertexHandle;
use crate::trimesh::TriMesh;
use crate::polymesh::PolyMesh;

/// Options controlling what data to read/write.
#[derive(Clone, Debug, Default)]
pub struct ObjOptions {
    pub vertex_normals: bool,
    pub vertex_colors: bool,
    pub face_normals: bool,
}

/// Result of reading an OBJ file.
#[derive(Clone, Debug, Default)]
pub struct ObjData {
    pub vertex_normals: Vec<Vector3<Scalar>>,
    pub face_normals: Vec<Vec<Vector3<Scalar>>>,
}

/// Errors from OBJ I/O.
#[derive(Debug)]
pub enum ObjError {
    Io(io::Error),
    Parse(String),
    Mesh(crate::error::MeshError),
}

impl From<io::Error> for ObjError {
    fn from(e: io::Error) -> Self {
        ObjError::Io(e)
    }
}

impl From<crate::error::MeshError> for ObjError {
    fn from(e: crate::error::MeshError) -> Self {
        ObjError::Mesh(e)
    }
}

impl std::fmt::Display for ObjError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjError::Io(e) => write!(f, "IO error: {}", e),
            ObjError::Parse(s) => write!(f, "Parse error: {}", s),
            ObjError::Mesh(e) => write!(f, "Mesh error: {}", e),
        }
    }
}

impl std::error::Error for ObjError {}

/// Parse a face vertex index component (1-based, possibly negative).
fn parse_face_index(s: &str, count: usize) -> Result<usize, ObjError> {
    let idx: i64 = s
        .parse()
        .map_err(|_| ObjError::Parse(format!("invalid index: {}", s)))?;
    if idx > 0 {
        Ok((idx - 1) as usize)
    } else if idx < 0 {
        // Negative indices count from end
        Ok((count as i64 + idx) as usize)
    } else {
        Err(ObjError::Parse("zero index in OBJ".into()))
    }
}

/// Read an OBJ file into a TriMesh (polygons are auto-triangulated).
pub fn read_trimesh_obj<P: AsRef<Path>>(path: P) -> Result<(TriMesh, ObjData), ObjError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut mesh = TriMesh::new();
    let mut data = ObjData::default();

    let mut positions: Vec<Point3<Scalar>> = Vec::new();
    let mut normals: Vec<Vector3<Scalar>> = Vec::new();
    let mut vertex_handles: Vec<VertexHandle> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') || line.starts_with("mtllib") || line.starts_with("usemtl") || line.starts_with('g') || line.starts_with('o') || line.starts_with('s') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "v" => {
                if parts.len() < 4 {
                    return Err(ObjError::Parse("vertex needs 3 coordinates".into()));
                }
                let x: Scalar = parts[1].parse().map_err(|_| ObjError::Parse("bad vertex x".into()))?;
                let y: Scalar = parts[2].parse().map_err(|_| ObjError::Parse("bad vertex y".into()))?;
                let z: Scalar = parts[3].parse().map_err(|_| ObjError::Parse("bad vertex z".into()))?;
                let p = Point3::new(x, y, z);
                positions.push(p);
                let vh = mesh.add_vertex(p);
                vertex_handles.push(vh);
            }
            "vn" => {
                if parts.len() < 4 {
                    return Err(ObjError::Parse("normal needs 3 components".into()));
                }
                let x: Scalar = parts[1].parse().map_err(|_| ObjError::Parse("bad normal".into()))?;
                let y: Scalar = parts[2].parse().map_err(|_| ObjError::Parse("bad normal".into()))?;
                let z: Scalar = parts[3].parse().map_err(|_| ObjError::Parse("bad normal".into()))?;
                normals.push(Vector3::new(x, y, z));
            }
            "vt" => {
                // Skip texture coordinates for now
            }
            "f" => {
                let nv = positions.len();
                let nn = normals.len();
                let mut face_verts: Vec<VertexHandle> = Vec::new();
                let mut face_normals: Vec<Vector3<Scalar>> = Vec::new();

                for &token in &parts[1..] {
                    let components: Vec<&str> = token.split('/').collect();

                    // Vertex index (required)
                    let vi = parse_face_index(components[0], nv)?;
                    face_verts.push(vertex_handles[vi]);

                    // Normal index (optional, in position 2: v//vn or v/vt/vn)
                    if components.len() == 3 && !components[2].is_empty() {
                        let ni = parse_face_index(components[2], nn)?;
                        face_normals.push(normals[ni]);
                    } else if components.len() == 2 {
                        // v/vt format - no normal
                    }
                }

                // TriMesh auto-triangulates
                mesh.add_face(&face_verts)?;

                if !face_normals.is_empty() {
                    data.face_normals.push(face_normals);
                }
            }
            _ => {
                // Ignore unknown lines
            }
        }
    }

    // Build per-vertex normals from face normal data if available
    if !normals.is_empty() {
        data.vertex_normals = vec![Vector3::zeros(); mesh.n_vertices()];
    }

    Ok((mesh, data))
}

/// Read an OBJ file into a PolyMesh (faces kept as polygons).
pub fn read_polymesh_obj<P: AsRef<Path>>(path: P) -> Result<(PolyMesh, ObjData), ObjError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut mesh = PolyMesh::new();
    let mut data = ObjData::default();

    let mut positions: Vec<Point3<Scalar>> = Vec::new();
    let mut normals: Vec<Vector3<Scalar>> = Vec::new();
    let mut vertex_handles: Vec<VertexHandle> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') || line.starts_with("mtllib") || line.starts_with("usemtl") || line.starts_with('g') || line.starts_with('o') || line.starts_with('s') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "v" => {
                if parts.len() < 4 {
                    return Err(ObjError::Parse("vertex needs 3 coordinates".into()));
                }
                let x: Scalar = parts[1].parse().map_err(|_| ObjError::Parse("bad vertex x".into()))?;
                let y: Scalar = parts[2].parse().map_err(|_| ObjError::Parse("bad vertex y".into()))?;
                let z: Scalar = parts[3].parse().map_err(|_| ObjError::Parse("bad vertex z".into()))?;
                let p = Point3::new(x, y, z);
                positions.push(p);
                let vh = mesh.add_vertex(p);
                vertex_handles.push(vh);
            }
            "vn" => {
                if parts.len() < 4 {
                    return Err(ObjError::Parse("normal needs 3 components".into()));
                }
                let x: Scalar = parts[1].parse().map_err(|_| ObjError::Parse("bad normal".into()))?;
                let y: Scalar = parts[2].parse().map_err(|_| ObjError::Parse("bad normal".into()))?;
                let z: Scalar = parts[3].parse().map_err(|_| ObjError::Parse("bad normal".into()))?;
                normals.push(Vector3::new(x, y, z));
            }
            "vt" => {}
            "f" => {
                let nv = positions.len();
                let nn = normals.len();
                let mut face_verts: Vec<VertexHandle> = Vec::new();
                let mut face_normals: Vec<Vector3<Scalar>> = Vec::new();

                for &token in &parts[1..] {
                    let components: Vec<&str> = token.split('/').collect();
                    let vi = parse_face_index(components[0], nv)?;
                    face_verts.push(vertex_handles[vi]);

                    if components.len() == 3 && !components[2].is_empty() {
                        let ni = parse_face_index(components[2], nn)?;
                        face_normals.push(normals[ni]);
                    }
                }

                mesh.add_face(&face_verts)?;

                if !face_normals.is_empty() {
                    data.face_normals.push(face_normals);
                }
            }
            _ => {}
        }
    }

    if !normals.is_empty() {
        data.vertex_normals = vec![Vector3::zeros(); mesh.n_vertices()];
    }

    Ok((mesh, data))
}

/// Write a TriMesh to an OBJ file.
pub fn write_trimesh_obj<P: AsRef<Path>>(
    path: P,
    mesh: &TriMesh,
    normals: Option<&[Vector3<Scalar>]>,
) -> Result<(), ObjError> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    writeln!(w, "# {} vertices, {} faces", mesh.n_vertices(), mesh.n_faces())?;

    // Write vertices
    for vh in mesh.vertices() {
        let p = mesh.point(vh);
        writeln!(w, "v {:.6} {:.6} {:.6}", p.x, p.y, p.z)?;
    }

    // Write normals
    if let Some(normals) = normals {
        for n in normals {
            writeln!(w, "vn {:.6} {:.6} {:.6}", n.x, n.y, n.z)?;
        }
    }

    // Write faces
    for fh in mesh.faces() {
        write!(w, "f")?;
        for vh in mesh.fv_ccw_iter(fh) {
            let vi = vh.idx() + 1; // OBJ is 1-based
            if normals.is_some() {
                write!(w, " {}//{}", vi, vi)?;
            } else {
                write!(w, " {}", vi)?;
            }
        }
        writeln!(w)?;
    }

    Ok(())
}

/// Write a PolyMesh to an OBJ file.
pub fn write_polymesh_obj<P: AsRef<Path>>(
    path: P,
    mesh: &PolyMesh,
    normals: Option<&[Vector3<Scalar>]>,
) -> Result<(), ObjError> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    writeln!(w, "# {} vertices, {} faces", mesh.n_vertices(), mesh.n_faces())?;

    for vh in mesh.vertices() {
        let p = mesh.point(vh);
        writeln!(w, "v {:.6} {:.6} {:.6}", p.x, p.y, p.z)?;
    }

    if let Some(normals) = normals {
        for n in normals {
            writeln!(w, "vn {:.6} {:.6} {:.6}", n.x, n.y, n.z)?;
        }
    }

    for fh in mesh.faces() {
        write!(w, "f")?;
        for vh in mesh.fv_ccw_iter(fh) {
            let vi = vh.idx() + 1;
            if normals.is_some() {
                write!(w, " {}//{}", vi, vi)?;
            } else {
                write!(w, " {}", vi)?;
            }
        }
        writeln!(w)?;
    }

    Ok(())
}
