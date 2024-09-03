use ecs_macros::{register_datagroup, CanCast};
use proto_ecs::systems::global_systems::register_global_system;

use crate::{core::{assets_management::models::Model, rendering::material::Material}, data_group::DataGroup, entities::{entity_system::{EntityMap, EntityPtr, World}, transform_datagroup::Transform}, systems::global_systems::GlobalSystem};

/// The render global system, used mostly to collect data from the entity system
/// and send it to the render
#[derive(Debug, CanCast)]
pub struct Render {

}

fn factory() -> Box<dyn GlobalSystem> {
    Box::new(Render{})
}

// Render Stage will be 250, almost the last
register_global_system!{
    Render,
    factory=factory,
    stages=(250),
    dependencies=(Transform, MeshRenderer),
}

impl RenderGlobalSystem for Render {
    fn stage_250(&mut self, world: &World, entity_map: &EntityMap, registered_entities: &Vec<EntityPtr>) {
        
    }
}

// Rendering local systems
#[derive(Debug, CanCast)]
pub struct MeshRenderer {
    material : Option<Material>,
    model : Option<Model>
}

fn mesh_renderer_factory() -> Box<dyn DataGroup> {
    return Box::new(MeshRenderer{material: None, model: None})
}

register_datagroup!{
    MeshRenderer,
    mesh_renderer_factory,
    init_style = NoArg
}

impl MeshRendererDesc for MeshRenderer {
    fn init(&mut self) {
        
    }
}