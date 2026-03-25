use std::fs::File;
use std::io::{BufWriter, Write};

use infmesh::polymesh::PolyMesh;
use infmesh::primitives::*;
use infmesh::trimesh::TriMesh;

/// Write multiple meshes into a single OBJ, offsetting each along the X axis.
fn main() {
    let path = "all_primitives.obj";
    let file = File::create(path).expect("failed to create file");
    let mut w = BufWriter::new(file);

    writeln!(w, "# All primitives").unwrap();

    let mut vertex_offset: usize = 0;
    let spacing = 1.5;
    let mut obj_index = 0;

    // Quad
    let (mesh, _) = Quad.generate().unwrap();
    write_polymesh(&mut w, &mesh, spacing * obj_index as f64, &mut vertex_offset, "quad").unwrap();
    obj_index += 1;

    // Circle (12 segments)
    let (mesh, _) = Circle { segments: 12 }.generate().unwrap();
    write_polymesh(&mut w, &mesh, spacing * obj_index as f64, &mut vertex_offset, "circle").unwrap();
    obj_index += 1;

    // Cube (1x1x1)
    let (mesh, _) = Cube { subdivisions: [1, 1, 1] }.generate().unwrap();
    write_polymesh(&mut w, &mesh, spacing * obj_index as f64, &mut vertex_offset, "cube").unwrap();
    obj_index += 1;

    // Cube (2x2x2 subdivisions)
    let (mesh, _) = Cube { subdivisions: [2, 2, 2] }.generate().unwrap();
    write_polymesh(&mut w, &mesh, spacing * obj_index as f64, &mut vertex_offset, "cube_subdivided").unwrap();
    obj_index += 1;

    // Icosphere (0 subdivisions)
    let (mesh, _) = Icosphere { subdivisions: 0 }.generate().unwrap();
    write_trimesh(&mut w, &mesh, spacing * obj_index as f64, &mut vertex_offset, "icosphere_0").unwrap();
    obj_index += 1;

    // Icosphere (2 subdivisions)
    let (mesh, _) = Icosphere { subdivisions: 2 }.generate().unwrap();
    write_trimesh(&mut w, &mesh, spacing * obj_index as f64, &mut vertex_offset, "icosphere_2").unwrap();
    obj_index += 1;

    // Cylinder (8 segments)
    let (mesh, _) = Cylinder {
        segments: 8,
        height: 1.0,
        radius: 0.5,
    }
    .generate()
    .unwrap();
    write_polymesh(&mut w, &mesh, spacing * obj_index as f64, &mut vertex_offset, "cylinder").unwrap();
    obj_index += 1;

    // UV Sphere (16 segments, 8 rings)
    let (mesh, _) = UvSphere {
        segments: 16,
        rings: 8,
    }
    .generate()
    .unwrap();
    write_polymesh(&mut w, &mesh, spacing * obj_index as f64, &mut vertex_offset, "uv_sphere").unwrap();

    println!("Wrote {} primitives to {}", obj_index + 1, path);
}

fn write_polymesh(
    w: &mut impl Write,
    mesh: &PolyMesh,
    x_offset: f64,
    vertex_offset: &mut usize,
    name: &str,
) -> std::io::Result<()> {
    writeln!(w, "\no {}", name)?;
    for vh in mesh.vertices() {
        let p = mesh.point(vh);
        writeln!(w, "v {:.6} {:.6} {:.6}", p.x + x_offset, p.y, p.z)?;
    }
    for fh in mesh.faces() {
        write!(w, "f")?;
        for vh in mesh.fv_ccw_iter(fh) {
            write!(w, " {}", vh.idx() + 1 + *vertex_offset)?;
        }
        writeln!(w)?;
    }
    *vertex_offset += mesh.n_vertices();
    Ok(())
}

fn write_trimesh(
    w: &mut impl Write,
    mesh: &TriMesh,
    x_offset: f64,
    vertex_offset: &mut usize,
    name: &str,
) -> std::io::Result<()> {
    writeln!(w, "\no {}", name)?;
    for vh in mesh.vertices() {
        let p = mesh.point(vh);
        writeln!(w, "v {:.6} {:.6} {:.6}", p.x + x_offset, p.y, p.z)?;
    }
    for fh in mesh.faces() {
        write!(w, "f")?;
        for vh in mesh.fv_ccw_iter(fh) {
            write!(w, " {}", vh.idx() + 1 + *vertex_offset)?;
        }
        writeln!(w)?;
    }
    *vertex_offset += mesh.n_vertices();
    Ok(())
}
