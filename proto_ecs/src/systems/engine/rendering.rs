use ecs_macros::{register_datagroup, CanCast};
use proto_ecs::systems::global_systems::register_global_system;

use crate::{
    core::{
        assets_management::models::ModelHandle,
        rendering::{
            camera::Camera,
            material::MaterialHandle,
            render_thread::{FrameDesc, RenderProxy, RenderThread},
        }, windowing::window_manager::WindowManager,
    },
    data_group::DataGroup,
    entities::{
        entity::EntityID,
        entity_system::{EntityMap, EntityPtr, World},
        transform_datagroup::Transform,
    },
    systems::global_systems::GlobalSystem,
};

/// The render global system, used mostly to collect data from the entity system
/// and send it to the render
#[derive(Debug, CanCast)]
pub struct Render {
    camera_entity: EntityID,
}

fn factory() -> Box<dyn GlobalSystem> {
    Box::new(Render { camera_entity: 0 })
}

impl Render {}

// Render Stage will be 250, almost the last
register_global_system! {
    Render,
    factory=factory,
    stages=(250),
    dependencies=(Transform, MeshRenderer),
}

impl RenderGlobalSystem for Render {
    fn stage_250(
        &mut self,
        world: &World,
        entity_map: &EntityMap,
        registered_entities: &Vec<EntityPtr>,
    ) {
        // If no camera, we have nothing to render
        if world.get_current_camera().is_none() {
            return;
        }

        // Update Frame Desc in render thread
        let next_frame_lock = RenderThread::get_next_frame_desc();
        let mut next_frame = next_frame_lock.write();

        // Update render proxies
        for (i, entity) in registered_entities.iter().enumerate() {
            let entity = entity.read();
            let transform = entity
                .get_datagroup::<Transform>()
                .expect("This entity should provide transforms");
            let mesh_renderer = entity
                .get_datagroup::<MeshRenderer>()
                .expect("This entity should provide a mesh renderer");

            // if no model, nothing to do with this entity
            if mesh_renderer.model.is_none() {
                continue;
            }
            if mesh_renderer.material.is_none() {
                unimplemented!("Should provide a default material when no material is provided");
            }

            let model = mesh_renderer.model.unwrap();
            let material = mesh_renderer.material.unwrap();
            let transform = transform.get_world_transform_mat();
            let new_proxy = RenderProxy {
                model,
                material,
                transform,
            };

            // If not enough render proxies currently in vector, add a new one
            if next_frame.render_proxies.len() == i + 1 {
                next_frame.render_proxies.push(new_proxy);
            } else {
                next_frame.render_proxies[i] = new_proxy;
            }
        }

        // Clear unused positions at the end of this vector
        next_frame
            .render_proxies
            .truncate(registered_entities.len());

        // Update the current camera
        let camera_id = world.get_current_camera().unwrap();
        let camera_lock = entity_map.get(&camera_id).expect("Camera no longer exists");
        let camera = camera_lock.read();
        let camera_dg = camera
            .get_datagroup::<CameraDG>()
            .expect("Camera entity should provide a CameraDG");
        next_frame.camera = camera_dg.camera;

        // Mark the next frame as ready to draw
        RenderThread::next_frame_updated();
    }
}

// Rendering local systems
#[derive(Debug, CanCast)]
pub struct MeshRenderer {
    material: Option<MaterialHandle>,
    model: Option<ModelHandle>,
}

fn mesh_renderer_factory() -> Box<dyn DataGroup> {
    return Box::new(MeshRenderer {
        material: None,
        model: None,
    });
}

register_datagroup! {
    MeshRenderer,
    mesh_renderer_factory,
    init_style = NoArg
}

impl MeshRendererDesc for MeshRenderer {
    fn init(&mut self) {}
}

// -- < Camera > ---------------------------------
#[derive(Debug, CanCast)]
pub struct CameraDG {
    camera: Camera,
}


fn camera_factory() -> Box<dyn DataGroup> {
    Box::new(CameraDG {
        camera: Camera::default(),
    })
}

register_datagroup! {
    CameraDG,
    camera_factory,
    init_style = NoArg
}

impl CameraDGDesc for CameraDG {
    fn init(&mut self) {}
}

#[derive(Debug, CanCast)]
pub struct CameraGS {
    initialized: bool,
}

fn camerags_factory() -> Box<dyn GlobalSystem> {
    Box::new(CameraGS { initialized: false })
}
register_global_system! {
    CameraGS,
    factory=camerags_factory,
    stages=(249),
    dependencies = (CameraDG),
}

impl CameraGSGlobalSystem for CameraGS {
    fn stage_249(
        &mut self,
        world: &World,
        _entity_map: &EntityMap,
        registered_entities: &Vec<EntityPtr>,
    ) {
        if registered_entities.is_empty() {
            return; // nothing to do without cameras to manage
        }

        // TODO Better camera management
        if !self.initialized {
            let mut entity = registered_entities[0].write();
            world.set_current_camera(entity.get_id());

            // Set up actual aspect ratio
            let camera_dg = entity.get_datagroup_mut::<CameraDG>().expect("Missing camera DG");
            let window_manager = WindowManager::get().read();
            let window = window_manager.get_window();
            let aspect_ratio = window.get_width() as f32 / window.get_heigth() as f32;
            camera_dg.camera.set_aspect_ratio(aspect_ratio);

            // mark as initialized
            self.initialized = true;
        }
    }
}
