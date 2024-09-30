use std::collections::HashMap;
use std::path::Path;

use macaw::{CoordinateSystem, Quat, Vec3, Vec3A};
use proto_ecs::core::assets_management::models::ModelHandle;
use proto_ecs::core::rendering::material::MaterialHandle;
use proto_ecs::core::rendering::render_thread::RenderThread;
use proto_ecs::core::windowing::events::Event;
use proto_ecs::core::windowing::window_manager::WindowManager;
use proto_ecs::entities::entity::EntityID;
use proto_ecs::entities::entity_spawn_desc::EntitySpawnDescription;
use proto_ecs::entities::entity_system::{EntitySystem, World};
use proto_ecs::entities::transform_datagroup::Transform;
use proto_ecs::prelude::*;
use proto_ecs::systems::engine::rendering::{CameraDG, CameraGS, MeshRenderer, RenderGS};
use proto_ecs::systems::local_systems::register_local_system;

struct MyLayer {
    material: Option<MaterialHandle>,
    model: Option<ModelHandle>,
}

// TODO Look for something to do in these cases
unsafe impl Send for MyLayer {}
unsafe impl Sync for MyLayer {}

impl Layer for MyLayer {
    fn on_attach(&mut self) {
        //  Set up resources required by this app
        self.load_resources();

        // Set up entities in this app
        self.set_up_entities();
    }

    fn on_detach(&mut self) {
        // TODO cleanup
    }

    fn update(&mut self, _delta_time: f32) {}

    fn imgui_update(&mut self, _delta_time: f32, ui: &mut imgui::Ui) {
        ui.window("Hello Triangle")
            .size([300.0, 300.0], imgui::Condition::FirstUseEver)
            .build(|| {});
    }

    fn on_event(&mut self, _event: &mut Event) {}
}

impl MyLayer {
    fn load_resources(&mut self) {
        let default = RenderThread::get_shader_handle_from_name("default")
            .expect("Default shader should be loaded by now");

        self.material = Some({
            Render::create_material(default, HashMap::new())
                .expect("Unable to create default material!")
        });

        self.model = Some({
            let path = Path::new("./resources/Cube/cube.obj");
            Render::get_or_load_model(path).expect("Should be able to load model")
        });
    }

    fn set_up_entities(&mut self) {
        let es = EntitySystem::get();
        debug_assert!(
            es.get_worlds_list().len() > 0,
            "World should be created by now"
        );
        let world = es.get_worlds_list()[0];

        // Create cube entity
        let mut entity_desc = EntitySpawnDescription::default();
        let mut transform = Transform::default();
        transform.set_local_rotation(Quat::from_euler(macaw::EulerRot::XYZ, 0.0, 45.0_f32.to_radians(), 0.0));
        Transform::prepare_spawn(&mut entity_desc, Box::new(transform));
        MeshRenderer::prepare_spawn(
            &mut entity_desc,
            Box::new(MeshRenderer::new(
                self.model.unwrap(),
                self.material.unwrap(),
            )),
        );
        RenderGS::simple_prepare(&mut entity_desc);
        ModelRotatorLS::simple_prepare(&mut entity_desc);

        entity_desc.set_name("Model entity".into());
        let _entity = es
            .create_entity(world, entity_desc)
            .expect("Could not create entity cube");

        // Create camera entity
        let mut camera_desc = EntitySpawnDescription::default();
        let mut init_camera = CameraDG::default();
        init_camera
            .get_camera_mut()
            .set_position(macaw::vec3a(0.0, 0.0, -10.0));

        init_camera.get_camera_mut().look_at(macaw::Vec3A::ZERO);
        init_camera
            .get_camera_mut()
            .set_up_vector(Vec3A::new(0.0, 1.0, 0.0));

        CameraDG::prepare_spawn(&mut camera_desc, Box::new(init_camera));
        CameraGS::simple_prepare(&mut camera_desc);

        camera_desc.set_name("Camera entity".into());
        let _camera_entity = es
            .create_entity(world, camera_desc)
            .expect("Should be able to create camera");
    }
}

struct ModelRotatorLS {}

register_local_system! {
    ModelRotatorLS,
    dependencies = (Transform),
    stages = (0),
    before = ()
}

impl ModelRotatorLSLocalSystem for ModelRotatorLS {
    fn stage_0(world: &World, entity_id: EntityID, transform: &mut Transform) {
        let delta_time = world.get_delta_time();
        let old_rotation = transform.get_local_rotation();
        let (_, rotation) = old_rotation.to_axis_angle();
        let mut new_angle = rotation + (delta_time as f32 * 25_f32).to_radians();
        if new_angle.to_degrees() >= 360.0
        {
            new_angle = 0.0;
        }
        let new_rot = Quat::from_axis_angle(Vec3::up(), new_angle);

        transform.set_local_rotation(new_rot);
    }
}

fn main() {
    App::initialize();
    WindowManager::init(
        WindowBuilder::new()
            .with_height(720)
            .with_width(720)
            .with_title("Sandbox Testing".to_owned()),
        Platforms::Windows,
    );
    Render::init();

    App::add_layer(Box::new(MyLayer {
        material: None,
        model: None,
    }));

    App::run_application();

    Render::shutdown();
}
