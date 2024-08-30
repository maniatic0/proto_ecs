use core::slice::{Iter, IterMut};
/// Layers implement user behavior. They provide an API  
/// that is called by the engine each iteration. A proto-ecs application
/// is basically a collection of layers provided by the user.
///
/// Layers take care of events and updates.
use proto_ecs::core::windowing::events::Event;
use scc::Queue;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;

pub type LayerPtr = Box<dyn Layer>;

pub trait Layer: Send + Sync {
    fn on_attach(&mut self);

    fn on_detach(&mut self);

    fn update(&mut self, delta_time: f32);

    fn on_event(&mut self, event: &mut Event);

    // Allow unused variables because this is just an empty default implementation.
    // Don't add _ to the start of their names so that the user has a good
    // autocompletion when implementing this function
    #[allow(unused)]
    fn imgui_update(&mut self, delta_time: f32, ui: &mut imgui::Ui) {}
}

pub struct LayerContainer {
    pub layer: LayerPtr,
    pub id: LayerID,
}
pub type LayerStack = Vec<LayerContainer>;
pub type LayerID = u32;

#[derive(Default)]
pub struct LayerManager {
    layers: LayerStack,
    overlays: LayerStack,
    layers_to_attach: Queue<(LayerID, LayerPtr)>,
    overlays_to_attach: Queue<(LayerID, LayerPtr)>,
    next_layer_id: AtomicU32,
    layers_to_detach: Queue<LayerID>,
    overlays_to_detach: Queue<LayerID>,
}

impl LayerManager {
    pub fn attach_layer(&mut self, layer: LayerPtr) -> LayerID {
        let id = self.next_layer_id.fetch_add(1, Ordering::Relaxed);
        self.layers_to_attach.push((id, layer));
        id
    }

    pub fn attach_overlays(&mut self, overlay: LayerPtr) -> LayerID {
        let id = self.next_layer_id.fetch_add(1, Ordering::Relaxed);
        self.overlays_to_attach.push((id, overlay));
        id
    }

    pub fn attach_pending_layers(&mut self) {
        while let Some(mut entry) = self.layers_to_attach.pop() {
            let (id, mut layer) = unsafe { entry.get_mut().unwrap().take_inner() };
            layer.on_attach();
            self.layers.push(LayerContainer { layer, id });
        }
    }

    pub fn attach_pending_overlays(&mut self) {
        while let Some(mut entry) = self.overlays_to_attach.pop() {
            let (id, mut layer) = unsafe { entry.get_mut().unwrap().take_inner() };
            layer.on_attach();
            self.overlays.push(LayerContainer { layer, id });
        }
    }

    pub fn detach_layer(&mut self, layer_id: &LayerID) {
        self.layers_to_detach.push(*layer_id);
    }

    pub fn detach_overlay(&mut self, layer_id: &LayerID) {
        self.overlays_to_detach.push(*layer_id);
    }

    pub fn detach_pending_overlays(&mut self) {
        let mut to_detach = vec![];
        while let Some(entry) = self.overlays_to_detach.pop() {
            let layer_id = **entry.as_ref();
            to_detach.push(layer_id);

            for overlay in self.overlays.iter_mut() {
                if overlay.id == layer_id {
                    overlay.layer.on_detach();
                }
            }
        }

        self.overlays.retain(|layer| !to_detach.contains(&layer.id));
    }

    pub fn detach_pending_layers(&mut self) {
        let mut to_detach = vec![];
        while let Some(entry) = self.layers_to_detach.pop() {
            let layer_id = **entry;
            to_detach.push(layer_id);

            for layer in self.layers.iter_mut() {
                if layer.id == layer_id {
                    layer.layer.on_detach();
                    break;
                }
            }
        }

        self.layers.retain(|layer| !to_detach.contains(&layer.id));
    }

    pub fn layers_iter(&self) -> Iter<LayerContainer> {
        self.layers.iter()
    }

    pub fn layers_iter_mut(&mut self) -> IterMut<LayerContainer> {
        self.layers.iter_mut()
    }

    pub fn overlays_iter(&mut self) -> Iter<LayerContainer> {
        self.overlays.iter()
    }

    pub fn overlays_iter_mut(&mut self) -> IterMut<LayerContainer> {
        self.overlays.iter_mut()
    }
}
