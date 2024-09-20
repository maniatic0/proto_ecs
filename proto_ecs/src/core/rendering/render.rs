use std::path::{Path, PathBuf};
use std::time::Duration;

use lazy_static::lazy_static;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::rendering::render_api::RenderCommand;
use proto_ecs::core::windowing::window_manager::WindowManager;

use crate::core::assets_management::models::{ModelHandle, ModelManager};
use crate::core::rendering::material::MaterialAllocator;
use crate::core::rendering::render_thread;
use crate::core::utils::handle::Handle;

use super::material::{Material, MaterialArguments, MaterialHandle};
use super::render_api::{ShaderHandle, API};
use super::render_thread::RenderThread;

#[derive(Debug, Default)]
pub struct SceneDescription {}

#[derive(Debug, Default)]
pub struct SceneData {}

pub struct Render {
    scene_begun: bool,
    pub(super) _scene_data: SceneData,
    pub(super) models: ModelManager,
    pub(super) materials: MaterialAllocator,
    render_thread: Option<std::thread::JoinHandle<()>>,
}

#[derive(Debug)]
pub enum RenderError {
    AssetNotFound {
        path: PathBuf,
    },
    InvalidAsset {
        handle: Handle,
        asset_type: AssetType,
    },
}

#[derive(Debug)]
pub enum AssetType {
    Shader,
    Material,
    Model,
}

lazy_static! {
    pub(super) static ref RENDER: RwLock<Option<Render>> = RwLock::default();
}

impl Render {
    pub fn init() {
        let mut render = RENDER.write();
        debug_assert!(render.is_none(), "Render already initialized");

        *render = Some(Render {
            scene_begun: false,
            _scene_data: SceneData::default(),
            models: ModelManager::default(),
            materials: MaterialAllocator::default(),
            render_thread: None,
        });

        println!("Starting Render Thread...");
        let render_handle = std::thread::Builder::new()
            .name("RenderThread".into())
            .spawn(|| {
                RenderCommand::initialize(WindowManager::get_platform());
                let mut render_thread = RenderThread::new();
                render_thread.start();
            })
            .expect("Could not start render thread");

        // Now that the render backend is initialized, start render thread
        let render = render.as_mut().unwrap();
        render.render_thread = Some(render_handle);
    }

    pub fn shutdown() {
        // Send a stop signal to the render thread
        RenderThread::stop();
        let mut render_lock = RENDER.write();
        let render = render_lock.as_mut().unwrap();
        let handle = render
            .render_thread
            .take()
            .expect("Render thread is not initialized");

        println!("Shutting down Render Thread...");
        match handle.join() {
            Ok(()) => println!("Render thread successfully finished"),
            Err(e) => eprintln!("Error in render thread: {:?}", e),
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
    pub fn get_or_load_model(path: &Path) -> Result<ModelHandle, RenderError> {
        let mut render_lock = RENDER.write();
        let render = render_lock.as_mut().expect("Render not yet initialized");
        if !path.exists() {
            return Err(RenderError::AssetNotFound { path: path.into() });
        }

        // Don't give users trivial access to the actual model
        let (_, handle) = render.models.get_or_load(&PathBuf::from(path));
        Ok(handle)
    }

    #[inline(always)]
    pub fn create_material(
        shader: ShaderHandle,
        params: MaterialArguments,
    ) -> Result<MaterialHandle, RenderError> {
        // Check that the material has a valid shader
        if !RenderCommand::shader_exists(shader) {
            return Err(RenderError::InvalidAsset {
                handle: shader,
                asset_type: AssetType::Shader,
            });
        }

        let mut render_lock = RENDER.write();
        let render = render_lock.as_mut().expect("Render not initialized");

        Ok(render.materials.allocate(Material {
            shader,
            parameters: params,
        }))
    }

    #[inline(always)]
    pub fn get_current_api() -> API {
        RenderCommand::get_current_api()
    }

    #[inline(always)]
    pub(super) fn get() -> &'static RENDER {
        &RENDER
    }
}
