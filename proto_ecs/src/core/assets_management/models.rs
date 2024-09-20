use proto_ecs::core::utils::handle::Allocator;
use std::collections::HashMap;
use tobj;

use std::path::PathBuf;
use std::fs::canonicalize;

use crate::core::utils::handle::Handle;

pub type ModelHandle = Handle;

#[derive(Default)]
pub struct ModelManager {
    model_allocator : Allocator<Model>,
    loaded_models : HashMap<PathBuf, ModelHandle>
}

#[derive(Debug)]
pub struct Model {
    internal_model : tobj::Model
}

impl ModelManager {
    pub fn is_model_loaded(&self, model_path : &PathBuf) -> bool {
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
    pub fn get(&self, model_handle : ModelHandle) -> &mut Model {
        self.model_allocator.get(model_handle)
    }

    pub fn get_or_load(&mut self, model_path : &PathBuf) -> (&mut Model, ModelHandle) {
        let canon_path = canonicalize(model_path).expect("Not a valid model path");
        if let Some(handle) = self.loaded_models.get(&canon_path) {
            let model = self.model_allocator.get(*handle);
            (model, *handle)
        }
        else {
            self.load(model_path)
        }
    }

    pub fn load(&mut self, model_path : &PathBuf) -> (&mut Model, ModelHandle) {
        debug_assert!(!self.loaded_models.contains_key(model_path), "Model is already loaded");

        let canon_path = canonicalize(model_path).expect("Invalid model file");
        let (mut models, materials)= tobj::load_obj(canon_path, &tobj::LoadOptions::default()).expect("Could not load model object");
        let _materials = materials.expect("Could not load model material");

        // for now we will only support models with a single pieace
        assert!(models.len() == 1);

        // We will also only care about the model itself and not materials since we don't have a 
        // good material system yet
        let handle = self.model_allocator.allocate(Model { internal_model: models.pop().unwrap() });

        (self.model_allocator.get(handle), handle)
    }

    pub fn unload(&mut self, model_handle : ModelHandle) {
        debug_assert!(self.model_allocator.is_live(model_handle), "Trying to unload unexistent model");

        // Clear from allocator, will free this model from memory
        self.model_allocator.free(model_handle);

        // Clear from map
        let mut model_path = None;
        for (path, handle)in self.loaded_models.iter() {
            if *handle == model_handle {
                model_path = Some(path.clone());
            }
        }

        let path = model_path.expect("Should exist in loaded models map");
        self.loaded_models.remove(&path);
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
}