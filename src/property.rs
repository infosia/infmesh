use std::any::Any;
use std::marker::PhantomData;

use crate::handle::{EdgeHandle, FaceHandle, HalfedgeHandle, VertexHandle};
use crate::error::MeshError;

/// A typed handle to a property stored in a `PropertyStore`.
#[derive(Debug)]
pub struct PropertyHandle<H, T> {
    id: usize,
    _phantom: PhantomData<(H, T)>,
}

impl<H, T> Clone for PropertyHandle<H, T> {
    fn clone(&self) -> Self { *self }
}
impl<H, T> Copy for PropertyHandle<H, T> {}

impl<H, T> PartialEq for PropertyHandle<H, T> {
    fn eq(&self, other: &Self) -> bool { self.id == other.id }
}
impl<H, T> Eq for PropertyHandle<H, T> {}

// Type aliases for convenience
pub type VPropHandle<T> = PropertyHandle<VertexHandle, T>;
pub type HPropHandle<T> = PropertyHandle<HalfedgeHandle, T>;
pub type EPropHandle<T> = PropertyHandle<EdgeHandle, T>;
pub type FPropHandle<T> = PropertyHandle<FaceHandle, T>;

/// Type-erased property storage for a single property.
trait PropertyStorage: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn resize_default(&mut self, new_len: usize);
    fn copy_element(&mut self, from: usize, to: usize);
    /// Compact storage using a mapping: map[old_index] = new_index.
    /// Entries where new_index == usize::MAX are deleted.
    fn compact(&mut self, map: &[usize]);
    fn name(&self) -> &str;
    fn clone_box(&self) -> Box<dyn PropertyStorage>;
}

/// Concrete typed property storage backed by a `Vec<T>`.
struct TypedProperty<T: Clone + Default + 'static> {
    data: Vec<T>,
    name: String,
}

impl<T: Clone + Default + 'static> PropertyStorage for TypedProperty<T> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn resize_default(&mut self, new_len: usize) {
        self.data.resize_with(new_len, T::default);
    }

    fn copy_element(&mut self, from: usize, to: usize) {
        let val = self.data[from].clone();
        self.data[to] = val;
    }

    fn compact(&mut self, map: &[usize]) {
        let mut new_len = 0usize;
        for i in 0..map.len().min(self.data.len()) {
            if map[i] != usize::MAX {
                if map[i] != i {
                    self.data[map[i]] = self.data[i].clone();
                }
                new_len = new_len.max(map[i] + 1);
            }
        }
        self.data.truncate(new_len);
    }

    fn name(&self) -> &str { &self.name }

    fn clone_box(&self) -> Box<dyn PropertyStorage> {
        Box::new(TypedProperty {
            data: self.data.clone(),
            name: self.name.clone(),
        })
    }
}

/// Identifies what kind of mesh element a property is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyKind {
    Vertex,
    Halfedge,
    Edge,
    Face,
}

/// Entry in the property store.
struct PropertyEntry {
    storage: Box<dyn PropertyStorage>,
    kind: PropertyKind,
}

impl Clone for PropertyEntry {
    fn clone(&self) -> Self {
        PropertyEntry {
            storage: self.storage.clone_box(),
            kind: self.kind,
        }
    }
}

/// Stores all properties for a mesh.
#[derive(Clone, Default)]
pub struct PropertyStore {
    entries: Vec<Option<PropertyEntry>>,
}

impl PropertyStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new property. Returns a typed handle.
    pub fn add<H: HandleKind + 'static, T: Clone + Default + 'static>(
        &mut self,
        name: &str,
        initial_size: usize,
    ) -> PropertyHandle<H, T> {
        let mut data = Vec::new();
        data.resize_with(initial_size, T::default);
        let storage = Box::new(TypedProperty { data, name: name.to_string() });
        let id = self.entries.len();
        self.entries.push(Some(PropertyEntry {
            storage,
            kind: H::KIND,
        }));
        PropertyHandle { id, _phantom: PhantomData }
    }

    /// Get an existing property handle by name.
    pub fn get_handle<H: HandleKind + 'static, T: Clone + Default + 'static>(
        &self,
        name: &str,
    ) -> Option<PropertyHandle<H, T>> {
        for (id, entry) in self.entries.iter().enumerate() {
            if let Some(e) = entry
                && e.kind == H::KIND
                && e.storage.name() == name
                && e.storage.as_any().downcast_ref::<TypedProperty<T>>().is_some()
            {
                return Some(PropertyHandle { id, _phantom: PhantomData });
            }
        }
        None
    }

    /// Remove a property.
    pub fn remove<H, T>(&mut self, handle: PropertyHandle<H, T>) {
        if handle.id < self.entries.len() {
            self.entries[handle.id] = None;
        }
    }

    /// Get a reference to a property value.
    pub fn get<H: Into<usize>, T: Clone + Default + 'static>(
        &self,
        handle: PropertyHandle<H, T>,
        elem: H,
    ) -> Result<&T, MeshError> {
        let entry = self.entries[handle.id].as_ref()
            .ok_or(MeshError::InvalidPropertyHandle)?;
        let typed = entry.storage.as_any().downcast_ref::<TypedProperty<T>>()
            .ok_or(MeshError::PropertyTypeMismatch)?;
        Ok(&typed.data[elem.into()])
    }

    /// Get a mutable reference to a property value.
    pub fn get_mut<H: Into<usize>, T: Clone + Default + 'static>(
        &mut self,
        handle: PropertyHandle<H, T>,
        elem: H,
    ) -> Result<&mut T, MeshError> {
        let entry = self.entries[handle.id].as_mut()
            .ok_or(MeshError::InvalidPropertyHandle)?;
        let typed = entry.storage.as_any_mut().downcast_mut::<TypedProperty<T>>()
            .ok_or(MeshError::PropertyTypeMismatch)?;
        Ok(&mut typed.data[elem.into()])
    }

    /// Set a property value.
    pub fn set<H: Into<usize>, T: Clone + Default + 'static>(
        &mut self,
        handle: PropertyHandle<H, T>,
        elem: H,
        value: T,
    ) -> Result<(), MeshError> {
        let entry = self.entries[handle.id].as_mut()
            .ok_or(MeshError::InvalidPropertyHandle)?;
        let typed = entry.storage.as_any_mut().downcast_mut::<TypedProperty<T>>()
            .ok_or(MeshError::PropertyTypeMismatch)?;
        typed.data[elem.into()] = value;
        Ok(())
    }

    /// Copy property value from one element to another.
    pub fn copy_element<H: Into<usize> + Copy, T: Clone + Default + 'static>(
        &mut self,
        handle: PropertyHandle<H, T>,
        from: H,
        to: H,
    ) -> Result<(), MeshError> {
        let entry = self.entries[handle.id].as_mut()
            .ok_or(MeshError::InvalidPropertyHandle)?;
        let typed = entry.storage.as_any_mut().downcast_mut::<TypedProperty<T>>()
            .ok_or(MeshError::PropertyTypeMismatch)?;
        let val = typed.data[from.into()].clone();
        typed.data[to.into()] = val;
        Ok(())
    }

    /// Resize all properties of a given kind to match the current element count.
    pub fn resize(&mut self, kind: PropertyKind, new_len: usize) {
        for e in self.entries.iter_mut().flatten() {
            if e.kind == kind {
                e.storage.resize_default(new_len);
            }
        }
    }

    /// Copy all properties from one element to another for a given kind.
    pub fn copy_all(&mut self, kind: PropertyKind, from: usize, to: usize) {
        for e in self.entries.iter_mut().flatten() {
            if e.kind == kind {
                e.storage.copy_element(from, to);
            }
        }
    }

    /// Compact all properties of a given kind using a handle mapping.
    /// `map[old_index]` gives the new index; `usize::MAX` means deleted.
    pub fn compact(&mut self, kind: PropertyKind, map: &[usize]) {
        for e in self.entries.iter_mut().flatten() {
            if e.kind == kind {
                e.storage.compact(map);
            }
        }
    }

    /// Number of properties of a given kind.
    pub fn n_properties(&self, kind: PropertyKind) -> usize {
        self.entries.iter()
            .filter(|e| e.as_ref().is_some_and(|e| e.kind == kind))
            .count()
    }

    /// Check if a named property exists.
    pub fn has_property(&self, kind: PropertyKind, name: &str) -> bool {
        self.entries.iter().any(|e| {
            e.as_ref().is_some_and(|e| e.kind == kind && e.storage.name() == name)
        })
    }
}

/// Trait to associate handle types with their property kind.
pub trait HandleKind {
    const KIND: PropertyKind;
}

impl HandleKind for VertexHandle {
    const KIND: PropertyKind = PropertyKind::Vertex;
}

impl HandleKind for HalfedgeHandle {
    const KIND: PropertyKind = PropertyKind::Halfedge;
}

impl HandleKind for EdgeHandle {
    const KIND: PropertyKind = PropertyKind::Edge;
}

impl HandleKind for FaceHandle {
    const KIND: PropertyKind = PropertyKind::Face;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_access_property() {
        let mut store = PropertyStore::new();
        let prop: VPropHandle<f64> = store.add("weight", 4);

        store.set(prop, VertexHandle::new(0), 1.5).unwrap();
        store.set(prop, VertexHandle::new(1), 2.5).unwrap();
        store.set(prop, VertexHandle::new(2), 3.5).unwrap();
        store.set(prop, VertexHandle::new(3), 4.5).unwrap();

        assert_eq!(*store.get(prop, VertexHandle::new(0)).unwrap(), 1.5);
        assert_eq!(*store.get(prop, VertexHandle::new(3)).unwrap(), 4.5);
    }

    #[test]
    fn get_property_by_name() {
        let mut store = PropertyStore::new();
        let prop: VPropHandle<f64> = store.add("weight", 4);

        let found: Option<VPropHandle<f64>> = store.get_handle("weight");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), prop);

        let not_found: Option<VPropHandle<f64>> = store.get_handle("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn remove_property() {
        let mut store = PropertyStore::new();
        let prop: VPropHandle<f64> = store.add("weight", 4);
        assert_eq!(store.n_properties(PropertyKind::Vertex), 1);

        store.remove(prop);
        assert_eq!(store.n_properties(PropertyKind::Vertex), 0);
    }

    #[test]
    fn copy_property_element() {
        let mut store = PropertyStore::new();
        let prop: VPropHandle<i32> = store.add("label", 4);

        store.set(prop, VertexHandle::new(0), 42).unwrap();
        store.copy_element(prop, VertexHandle::new(0), VertexHandle::new(2)).unwrap();

        assert_eq!(*store.get(prop, VertexHandle::new(2)).unwrap(), 42);
    }

    #[test]
    fn resize_property() {
        let mut store = PropertyStore::new();
        let prop: VPropHandle<f64> = store.add("weight", 4);

        store.resize(PropertyKind::Vertex, 8);
        // Should be able to access extended elements (default value)
        assert_eq!(*store.get(prop, VertexHandle::new(7)).unwrap(), 0.0);
    }

    #[test]
    fn multiple_property_types() {
        let mut store = PropertyStore::new();
        let vprop: VPropHandle<f64> = store.add("vweight", 4);
        let fprop: FPropHandle<i32> = store.add("flabel", 2);

        store.set(vprop, VertexHandle::new(0), 1.0).unwrap();
        store.set(fprop, FaceHandle::new(0), 99).unwrap();

        assert_eq!(*store.get(vprop, VertexHandle::new(0)).unwrap(), 1.0);
        assert_eq!(*store.get(fprop, FaceHandle::new(0)).unwrap(), 99);
    }

    #[test]
    fn bool_property() {
        let mut store = PropertyStore::new();
        let prop: VPropHandle<bool> = store.add("selected", 4);

        store.set(prop, VertexHandle::new(1), true).unwrap();
        assert!(!*store.get(prop, VertexHandle::new(0)).unwrap());
        assert!(*store.get(prop, VertexHandle::new(1)).unwrap());
    }
}
