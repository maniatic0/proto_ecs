use lazy_static::lazy_static;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::rendering::render_api::RenderCommand;
use proto_ecs::core::windowing::window_manager::WindowManager;

use crate::core::assets_management::models::ModelManager;
use crate::core::rendering::material::MaterialAllocator;

use super::render_api::API;
use super::render_thread::RenderThread;

#[derive(Debug, Default)]
pub struct SceneDescription {}

#[derive(Debug, Default)]
pub struct SceneData {}

pub struct Render {
    scene_begun: bool,
    _scene_data: SceneData,
    models: ModelManager,
    materials: MaterialAllocator,
    render_thread : Option<std::thread::JoinHandle<()>>
}

pub enum RenderError {}

lazy_static! {
    static ref RENDER: RwLock<Option<Render>> = RwLock::default();
}

impl Render {
    pub fn init() {
        let mut render = RENDER.write();
        debug_assert!(render.is_none(), "Render already initialized");

        println!("Starting Render Thread...");
        RenderThread::init();
        let render_handle = std::thread::spawn(RenderThread::run);
        *render = Some(Render {
            scene_begun: false,
            _scene_data: SceneData::default(),
            models: ModelManager::default(),
            materials: MaterialAllocator::default(),
            render_thread: Some(render_handle)
        });
        RenderCommand::initialize(WindowManager::get_platform());
    }

    pub fn shutdown() {
        // Send a stop signal to the render thread
        RenderThread::stop();
        let mut render_lock = RENDER.write();
        let render = render_lock.as_mut().unwrap();
        let handle = render.render_thread.take().expect("Render thread is not initialized");

        println!("Shutting down Render Thread...");
        match handle.join() {
            Ok(()) => println!("Render thread successfully finished"),
            Err(e) => eprintln!("Error in render thread: {:?}", e)
        }
    }

    pub fn begin_scene(_scene_description: &SceneDescription) {
        let mut render_ref = RENDER.write();
        let render = render_ref.as_mut().expect("Render not yet initialized");

        render.scene_begun = true;
        // TODO
    }

    pub fn end_scene() {
        let mut render_ref = RENDER.write();
        let render = render_ref.as_mut().expect("Render not yet initialized");

        render.scene_begun = false;
        // TODO
    }

    pub fn on_window_resize(new_width: u32, new_height: u32) {
        // ? Not sure what the x,y parameters mean. They come from the OpenGL API
        RenderCommand::set_viewport(0, 0, new_width, new_height);
    }

    #[inline(always)]
    pub fn get_model_manager(&self) -> &ModelManager {
        &self.models
    }

    #[inline(always)]
    pub fn get_model_manager_mut(&mut self) -> &mut ModelManager {
        &mut self.models
    }

    #[inline(always)]
    pub fn get_materials(&self) -> &MaterialAllocator {
        &self.materials
    }

    #[inline(always)]
    pub fn get_materials_mut(&mut self) -> &mut MaterialAllocator {
        &mut self.materials
    }

    #[inline(always)]
    pub fn get_current_api() -> API {
        RenderCommand::get_current_api()
    }
}
