use proto_ecs::core::utils::handle::Allocator;
use std::collections::HashMap;
use tobj;

use std::fs::canonicalize;
use std::path::{PathBuf, Path};

use crate::core::utils::handle::Handle;

pub type ModelHandle = Handle;

#[derive(Default)]
pub struct ModelManager {
    model_allocator: Allocator<Model>,
    loaded_models: HashMap<PathBuf, Vec<ModelHandle>>,
}

#[derive(Debug)]
pub struct Model {
    internal_model: tobj::Model,
}

impl ModelManager {
    pub fn is_model_loaded(&self, model_path: &PathBuf) -> bool {
        if !model_path.exists() {
            panic!("Invalid Path: file {:?} does not exists", model_path);
        }
        let path = canonicalize(model_path).expect("Invalid file");

        self.loaded_models.contains_key(&path)
    }

    pub fn is_model_loaded_handle(&self, model_handle: ModelHandle) -> bool {
        self.model_allocator.is_live(model_handle)
    }

    #[inline(always)]
    pub fn get(&self, model_handle: ModelHandle) -> &mut Model {
        self.model_allocator.get(model_handle)
    }

    pub fn get_or_load(&mut self, model_path: &PathBuf) -> Vec<ModelHandle> {
        let canon_path = canonicalize(model_path).expect("Not a valid model path");
        if let Some(handles) = self.loaded_models.get(&canon_path) {
            let mut result = vec![];
            for handle in handles {
                let model = self.model_allocator.get(*handle);
                result.push(*handle);
            }
            result
        } else {
            self.load(model_path)
        }
    }

    pub fn load(&mut self, model_path: &PathBuf) -> Vec<ModelHandle> {
        debug_assert!(
            !self.loaded_models.contains_key(model_path),
            "Model is already loaded"
        );

        let canon_path = canonicalize(model_path).expect("Invalid model file");

        // Actually load the model
        let load_options = tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        };

        let (models, materials) =
            tobj::load_obj(canon_path, &load_options).expect("Could not load model object");
        let _materials = materials.expect("Could not load model material");

        // We will also only care about the model itself and not materials since we don't have a
        // good material system yet

        let mut result = vec![];

        for model in models {
            let handle = self.model_allocator.allocate(Model {
                internal_model: model,
            });

            result.push(handle);
        }

        result
    }

    pub fn unload(&mut self, model_handle: ModelHandle) {
        debug_assert!(
            self.model_allocator.is_live(model_handle),
            "Trying to unload unexistent model"
        );

        // Clear from allocator, will free this model from memory
        self.model_allocator.free(model_handle);

        // Clear from map
        let mut model_path = None;
        let mut delete_path = false;
        for (path, handles) in self.loaded_models.iter_mut() {
            let mut handle_index = None; 
            for (i, handle )in handles.iter().enumerate() {
                if *handle == model_handle {
                    model_path = Some(path.clone());
                    handle_index = Some(i);
                    break;
                }
            }

            if let Some(i) = handle_index {
                handles.remove(i);
                delete_path = handles.is_empty();
            }
        }

        // Checks if the model was actually loaded
        let path = model_path.expect("Should exist in loaded models map");

        // Remove if no models are left for this path
        if delete_path {
            self.loaded_models.remove(&path);
        }
    }

    fn unload_from_path(&mut self, model_path : &Path) {
        debug_assert!(self.loaded_models.contains_key(model_path), "Trying to unload unloaded model");
        let models = self.loaded_models.get(model_path);
        let handles = models.as_ref().unwrap();
        for handle in handles.iter() {
            self.model_allocator.free(*handle);
        }

        self.loaded_models.remove(model_path);
    }
}

impl Model {
    pub fn vertices(&self) -> &[f32] {
        // TODO we have to make this buffer to hold the entire data for the object,
        // not just the positions. We also have to provide a layout
        &self.internal_model.mesh.positions
    }

    pub fn indices(&self) -> &[u32] {
        &self.internal_model.mesh.indices
    }

    /// Return the entire model data in a vector.
    /// The order of the following properties, if present, is as
    /// follows:
    ///     1. Positions
    ///     2. normals
    ///     3. UVs
    pub fn data(&self) -> Vec<f32> {
        let capacity = {
            let vertices = self.internal_model.mesh.positions.len();
            let normals = self.internal_model.mesh.normals.len();
            let uvs = self.internal_model.mesh.texcoords.len();

            vertices + normals + uvs
        };

        let mut result = Vec::with_capacity(capacity);
        let n_vertices = self.internal_model.mesh.positions.len() / 3;

        for i in 0..n_vertices {
            let base = i * 3;
            let uv_base = i * 2;

            // Positions
            result.push(self.internal_model.mesh.positions[base]);
            result.push(self.internal_model.mesh.positions[base + 1]);
            result.push(self.internal_model.mesh.positions[base + 2]);

            // Normals
            result.push(self.internal_model.mesh.normals[base]);
            result.push(self.internal_model.mesh.normals[base + 1]);
            result.push(self.internal_model.mesh.normals[base + 2]);

            // UVs
            result.push(self.internal_model.mesh.texcoords[uv_base]);
            result.push(self.internal_model.mesh.texcoords[uv_base + 1]);
        }

        result
    }
}
