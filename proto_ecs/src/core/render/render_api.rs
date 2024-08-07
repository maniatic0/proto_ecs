use lazy_static::lazy_static;
use proto_ecs::core::math::Color;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::platform::Platforms;
use proto_ecs::core::platform::opengl::opengl_render_backend::OpenGLRenderBackend;
use super::vertex_array::{VertexArrayDyn, VertexArrayPtr};

pub enum API {
    OpenGL,
    Vulkan,
    None
}

/// This is the behaviour that a render api instance should implement,
/// translating the platform-specific details of the API to this trait
pub trait RenderAPIBackendDyn : Send + Sync {
    fn init(&mut self);
    fn clear_color(&self);
    fn set_clear_color(&mut self, color : Color);
    fn get_api(&self) -> API;
    fn set_viewport(&mut self, x : u32, y : u32, width : u32, height : u32);
    fn draw_indexed(&mut self, vertex_array : &Box<dyn VertexArrayDyn>);
}

/// Implement this trait to support a new Render API
pub trait RenderAPIBackend : RenderAPIBackendDyn {
    fn create() -> RenderAPIBackendPtr;
}

pub type RenderAPIBackendPtr = Box<dyn RenderAPIBackendDyn>;

lazy_static!{
    static ref RENDER_API : RwLock<RenderCommand> = RwLock::new(RenderCommand{backend: None});
}

/// RenderCommand is a class we use to interface with the currently used backend
pub struct RenderCommand {
    backend : Option<RenderAPIBackendPtr>
}

impl RenderCommand {
    pub fn initialize(platform: Platforms) {
        let mut render_api = RENDER_API.write();
        assert!(render_api.backend.is_none(), "Render api already initialized");
        match platform {
            Platforms::Windows => {
                render_api.backend = Some(OpenGLRenderBackend::create());
            }
            _ => panic!("Platform Render API backend not yet implemented")
        }
    }

    fn get_backend(&self) -> &RenderAPIBackendPtr{
        debug_assert!(self.backend.is_some(), "render api not initialized!");
        self.backend.as_ref().unwrap()
    }

    fn get_backend_mut(&mut self) -> &mut RenderAPIBackendPtr{
        debug_assert!(self.backend.is_some(), "render api not initialized!");
        self.backend.as_mut().unwrap()
    }

    pub fn draw_indexed(vertex_array : &VertexArrayPtr) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();

        vertex_array.bind();
        backend.draw_indexed(vertex_array);
    }

    pub fn clear() {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.clear_color();
    }

    pub fn set_clear_color(color : Color) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_clear_color(color);
    }

    pub fn set_viewport(x : u32, y : u32, width : u32, height : u32) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_viewport(x, y, width, height);
    }

    pub fn get_current_api() -> API {
        let api = RENDER_API.read();
        let backend = api.get_backend();
        backend.get_api()
    }
}