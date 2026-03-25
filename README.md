# infmesh

Unofficial Rust port of the [OpenMesh](https://www.graphics.rwth-aachen.de/software/infmesh/) halfedge mesh library. Not a direct C++ translation or FFI wrapper -- built from the ground up using idiomatic Rust patterns.

## Features

- **Halfedge data structure** -- vertices, edges, halfedges, and faces with full connectivity traversal
- **Handle-based element access** -- lightweight `Copy` handle types (`VertexHandle`, `EdgeHandle`, `HalfedgeHandle`, `FaceHandle`)
- **Iterators** -- linear iterators over all elements and circulators for traversing rings (vertex-vertex, vertex-face, face-vertex, etc.)
- **Triangle and polygon meshes** -- `TriMesh` (auto-triangulation, flip, split, collapse) and `PolyMesh` (arbitrary polygons)
- **Polygon mesh operations** -- extrude faces, extrude edges, insert edge, loop cut, split edge, flip face normals, edge ring selection
- **Primitives** -- generate common shapes: Quad, Circle, Cube, Cylinder, Icosphere, UV Sphere
- **Geometry** -- normals, centroids, areas, edge lengths, dihedral angles via `nalgebra`
- **Property system** -- attach typed per-element data (vertex colors, face labels, etc.)
- **Deletion and garbage collection** -- lazy deletion with compacting garbage collection
- **OBJ I/O** -- read and write Wavefront OBJ files
- **Algorithms** -- Loop and Catmull-Clark subdivision, Laplacian/Taubin smoothing, quadric error metric decimation

## Quick Start

```toml
[dependencies]
infmesh = { path = "." }
nalgebra = "0.33"
```

### Create a mesh

```rust
use infmesh::TriMesh;
use nalgebra::Point3;

let mut mesh = TriMesh::new();

let v0 = mesh.add_vertex(Point3::new(0.0, 0.0, 0.0));
let v1 = mesh.add_vertex(Point3::new(1.0, 0.0, 0.0));
let v2 = mesh.add_vertex(Point3::new(0.5, 1.0, 0.0));
let v3 = mesh.add_vertex(Point3::new(0.5, 0.5, 1.0));

let faces = [
    &[v0, v1, v2] as &[_],
    &[v0, v2, v3],
    &[v0, v3, v1],
    &[v1, v3, v2],
];
for verts in &faces {
    match mesh.add_face(verts) {
        Ok(_fh) => println!("Added face"),
        Err(e) => eprintln!("Failed to add face: {e}"),
    }
}
```

### Traverse connectivity

```rust
// Iterate over all vertices
for vh in mesh.vertices() {
    let p = mesh.point(vh);
    println!("Vertex {}: ({}, {}, {})", vh.idx(), p.x, p.y, p.z);
}

// Circulate around a vertex (vertex-vertex ring)
for neighbor in mesh.vv_cw_iter(v0) {
    println!("Neighbor: {}", neighbor.idx());
}

// Iterate over face vertices
for fh in mesh.faces() {
    let verts: Vec<_> = mesh.fv_ccw_iter(fh).collect();
    println!("Face {}: {:?}", fh.idx(), verts.iter().map(|v| v.idx()).collect::<Vec<_>>());
}
```

### Geometry calculations

```rust
use infmesh::geometry;

let normal = geometry::calc_face_normal(&mesh, fh);
let centroid = geometry::calc_face_centroid(&mesh, fh);
let area = geometry::calc_face_area(&mesh, fh);
let angle = geometry::calc_dihedral_angle(&mesh, eh);
```

### Read/write OBJ files

```rust
use infmesh::io::obj;

let (mesh, _data) = match obj::read_trimesh_obj("model.obj") {
    Ok(result) => result,
    Err(e) => {
        eprintln!("Failed to read OBJ: {e}");
        return;
    }
};

if let Err(e) = obj::write_trimesh_obj("output.obj", &mesh, None) {
    eprintln!("Failed to write OBJ: {e}");
}
```

### Primitives

```rust
use infmesh::{Primitive, Cube, Cylinder, UvSphere, Icosphere, Quad, Circle};

// Unit cube with 2×2×2 subdivisions per face
let (mesh, data) = Cube { subdivisions: [2, 2, 2] }.generate()?;

// Cylinder with 8 sides
let (mesh, data) = Cylinder { segments: 8, height: 1.0, radius: 0.5 }.generate()?;

// UV sphere
let (mesh, data) = UvSphere { segments: 16, rings: 8 }.generate()?;

// Icosphere (returns a TriMesh)
let (mesh, data) = Icosphere { subdivisions: 2 }.generate()?;

// Simple shapes
let (mesh, _) = Quad.generate()?;
let (mesh, _) = Circle { segments: 12 }.generate()?;
```

### Polygon mesh operations

```rust
use infmesh::PolyMesh;

// Extrude faces outward
let result = mesh.extrude_faces(&[face])?;
// Move extruded vertices along normal
for &v in &result.new_vertices {
    let p = mesh.point(v);
    mesh.set_point(v, p + normal * 0.5);
}

// Insert an edge across a face to split it in two
let result = mesh.insert_edge(face, va, vb)?;

// Loop cut -- split faces along an edge ring
let result = mesh.loop_cut(edge)?;

// Split an edge at its midpoint
let result = mesh.split_edge(edge)?;

// Extrude boundary edges outward
let result = mesh.extrude_edges(&[edge])?;

// Flip face normals (reverse winding order)
let new_faces = mesh.flip_face_normals(&[face])?;

// Query the edge ring passing through an edge
let ring = mesh.find_edge_ring(edge);
```

### Algorithms

```rust
use infmesh::algo::{loop_subdivision, catmull_clark, smoother, decimater};

// Subdivide (TriMesh)
loop_subdivision::loop_subdivide_n(&mut mesh, 2);

// Subdivide (PolyMesh)
catmull_clark::catmull_clark_subdivide(&mut poly_mesh);

// Smooth
let opts = smoother::SmoothOptions {
    iterations: 5,
    lambda: 0.5,
    fix_boundary: true,
};
smoother::laplacian_smooth(&mut mesh, &opts);

// Decimate to target vertex count
decimater::decimate(&mut mesh, 100);
```

## Project Structure

```
src/
  lib.rs              -- crate root, module declarations and re-exports
  handle.rs           -- handle newtypes (VertexHandle, etc.) and Status bitflags
  error.rs            -- MeshError enum
  mesh/
    mod.rs
    connectivity.rs   -- halfedge data structure, topology, deletion, garbage collection
    iterators.rs      -- linear iterators (vertices, edges, faces, halfedges)
    circulators.rs    -- ring iterators (vv, vf, fv, fh, etc.)
    tri_ops.rs        -- triangle-specific ops (flip, split, collapse)
    poly_ops.rs       -- polygon ops (extrude faces/edges, insert edge, loop cut, split edge, flip normals, edge ring)
  primitives.rs       -- shape generators (Quad, Circle, Cube, Cylinder, Icosphere, UvSphere)
  property.rs         -- typed per-element property maps
  trimesh.rs          -- TriMesh with geometry
  polymesh.rs         -- PolyMesh with geometry
  geometry.rs         -- normals, centroids, areas, angles
  io/
    obj.rs            -- Wavefront OBJ reader/writer
  algo/
    loop_subdivision.rs  -- Loop subdivision (TriMesh)
    catmull_clark.rs     -- Catmull-Clark subdivision (PolyMesh)
    smoother.rs          -- Laplacian and Taubin smoothing
    decimater.rs         -- quadric error metric decimation
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `nalgebra` | Points, vectors, matrices |
| `bitflags` | Element status flags |
| `thiserror` | Error types |
| `approx` (dev) | Floating-point test comparisons |

## Testing

```sh
cargo test
```

129 tests covering handles, connectivity, iterators, circulators, properties, deletion, triangle operations, polygon operations, primitives, geometry, I/O, and algorithms.

## Improvements over C++ OpenMesh

This port fixes several issues present in the C++ OpenMesh implementation:

- **`is_collapse_ok` cleans up tag state** -- The C++ version tags neighboring vertices during the one-ring intersection test but never clears the `TAGGED` status bits afterward, leaving polluted state for any downstream code that relies on clean tags. The Rust version clears all tags before returning.
- **`collapse_loop` sets all next/prev links** -- When splicing a halfedge into a neighboring chain during 2-gon collapse, the C++ version only sets 2 of the 4 required next/prev links. The Rust version correctly sets all 4, preventing potential connectivity corruption.
- **Garbage collection compacts geometry** -- `TriMesh` and `PolyMesh` override garbage collection to compact the `points` vector alongside the topology arrays, ensuring vertex positions stay in sync with remapped handles.
- **Decimater guards against NaN errors** -- The quadric error decimater handles NaN error values gracefully: `CollapseCandidate` ordering treats NaN consistently to avoid `BinaryHeap` invariant violations, and NaN-error candidates are skipped during collapse selection.

## References

- [OpenMesh](https://www.graphics.rwth-aachen.de/software/openmesh/) -- original C++ library
- C. T. Loop, "Smooth Subdivision Surfaces Based on Triangles", 1987
- E. Catmull & J. Clark, "Recursively Generated B-spline Surfaces on Arbitrary Topological Meshes", 1978
- Garland & Heckbert, "Surface Simplification Using Quadric Error Metrics", 1997

## License

MIT
