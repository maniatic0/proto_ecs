use ecs_macros::{register_datagroup, CanCast};
use proto_ecs::systems::global_systems::register_global_system;

use crate::{
    core::{
        assets_management::models::ModelHandle,
        rendering::{
            camera::Camera,
            material::MaterialHandle,
            render_thread::{RenderProxy, RenderThread},
        }, windowing::window_manager::WindowManager,
    },
    data_group::{DataGroup, GenericDataGroupInitArgTrait},
    entities::{
        entity::EntityID,
        entity_system::{EntityMap, EntityPtr, World},
        transform_datagroup::Transform,
    },
    systems::global_systems::{GSLifetime, GlobalSystem},
};

/// The render global system, used mostly to collect data from the entity system
/// and send it to the render
#[derive(Debug, CanCast)]
pub struct RenderGS {
    _camera_entity: EntityID,
    /// TODO This is a workaround while we create lifetime functions (init, update, destroy)
    _initialized: bool 
}

fn factory() -> Box<dyn GlobalSystem> {
    Box::new(RenderGS { _camera_entity: 0, _initialized : false })
}

impl RenderGS {}

// Render Stage will be 250, almost the last
register_global_system! {
    RenderGS,
    factory=factory,
    stages=(250),
    dependencies=(Transform, MeshRenderer),
    lifetime = GSLifetime::AlwaysLive
}

impl RenderGSGlobalSystem for RenderGS {
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
        let mut n_proxies = 0;
        for entity in registered_entities.iter() {
            let entity = entity.read();
            let transform = entity
                .get_datagroup::<Transform>()
                .expect("This entity should provide transforms");
            let mesh_renderer = entity
                .get_datagroup::<MeshRenderer>()
                .expect("This entity should provide a mesh renderer");

            // if no model, nothing to do with this entity
            if mesh_renderer.models.is_empty() {
                continue;
            }
            if mesh_renderer.materials.is_empty() {
                unimplemented!("Should provide a default material when no material is provided");
            }

            let models = &mesh_renderer.models;
            let materials = &mesh_renderer.materials;
            debug_assert!(models.len() == materials.len(), "Each model should provide a material");

            let transform_mat = transform.get_world_transform_mat();
            for (model, material) in models.iter().zip(materials.iter()) {
                let new_proxy = RenderProxy {
                    model: *model,
                    material: *material,
                    transform: transform_mat,
                    position: *transform.get_world_positon()
                };

                // If not enough render proxies currently in vector, add a new one
                if next_frame.render_proxies.len() == n_proxies {
                    next_frame.render_proxies.push(new_proxy);
                } else {
                    next_frame.render_proxies[n_proxies] = new_proxy;
                }
                n_proxies += 1;
            }
        }

        // Clear unused positions at the end of this vector
        next_frame
            .render_proxies
            .truncate(n_proxies);

        // Update the current camera
        let camera_id = world.get_current_camera().unwrap();
        let camera_lock = entity_map.get(&camera_id).expect("Camera no longer exists");
        let camera = camera_lock.read();
        let camera_dg = camera
            .get_datagroup::<CameraDG>()
            .expect("Camera entity should provide a CameraDG");
        next_frame.camera = camera_dg.camera;

        // Mark the next frame as ready to draw
        // RenderThread::next_frame_updated();
    }
}

// Rendering local systems
#[derive(Debug, CanCast)]
pub struct MeshRenderer {
    materials: Vec<MaterialHandle>,
    models: Vec<ModelHandle>,
}

fn mesh_renderer_factory() -> Box<dyn DataGroup> {
    Box::new(MeshRenderer {
        materials: vec![],
        models: vec![],
    })
}

register_datagroup! {
    MeshRenderer,
    mesh_renderer_factory,
    init_style = Arg(MeshRenderer)
}

impl MeshRendererDesc for MeshRenderer {
    fn init(&mut self,init_data: std::boxed::Box<MeshRenderer>) {
        self.models = init_data.models;
        self.materials = init_data.materials;
    }
}

impl GenericDataGroupInitArgTrait for MeshRenderer {}

impl MeshRenderer {
    pub fn new(models : Vec<ModelHandle>, materials : Vec<MaterialHandle>) -> Self {
        MeshRenderer{
            models, materials 
        }
    }
}

// -- < Camera > ---------------------------------
#[derive(Debug, CanCast, Default)]
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
    init_style = Arg(CameraDG)
}

impl CameraDGDesc for CameraDG {
    fn init(&mut self,init_data:std::boxed::Box<CameraDG>) {
        self.camera = init_data.camera;
    }
}

impl GenericDataGroupInitArgTrait for CameraDG {}

impl CameraDG {
    
    #[inline(always)]
    pub fn get_camera(&self) -> &Camera {
        &self.camera
    }

    #[inline(always)]
    pub fn get_camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }
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
            let entity_id = {
                let entity = registered_entities[0].read();
                entity.get_id()
            };

            world.set_current_camera(entity_id);
            let mut entity = registered_entities[0].write();

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
