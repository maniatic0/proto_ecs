use super::buffer::BufferLayout;
use super::shader::ShaderDataType;
use super::vertex_array::VertexArrayDyn;
use lazy_static::lazy_static;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::math::Color;
use proto_ecs::core::platform::opengl::opengl_render_backend::OpenGLRenderBackend;
use proto_ecs::core::platform::Platforms;
use proto_ecs::core::rendering::handle::Handle;

pub type VertexBufferHandle = Handle;
pub type IndexBufferHandle = Handle;
pub type VertexArrayHandle = Handle;
pub type ShaderHandle = Handle;


pub enum API {
    OpenGL,
    Vulkan,
    None,
}

/// This is the behaviour that a render api instance should implement,
/// translating the platform-specific details of the API to this trait
pub trait RenderAPIBackendDyn: Send + Sync {
    fn init(&mut self);
    fn clear_color(&self);
    fn set_clear_color(&mut self, color: Color);
    fn get_api(&self) -> API;
    fn set_viewport(&mut self, x: u32, y: u32, width: u32, height: u32);
    fn draw_indexed(&mut self, vertex_array: &dyn VertexArrayDyn);

    // Resource creation and destruction
    fn create_vertex_buffer(&mut self, vertex_data : &[f32]) -> VertexBufferHandle;
    fn destroy_vertex_buffer(&mut self, handle : &VertexBufferHandle);
    fn create_index_buffer(&mut self, indices : &[u32]) -> IndexBufferHandle;
    fn destroy_index_buffer(&mut self, handle : &IndexBufferHandle);
    fn create_vertex_array(&mut self) -> VertexArrayHandle;
    fn destroy_vertex_array(&mut self, handle : &VertexArrayHandle);
    fn create_shader(&mut self, name : &str, vertex_src : &str, fragment_src : &str) -> ShaderHandle;
    fn destroy_shader(&mut self, handle : &ShaderHandle);

    // Bindings 
    fn bind_vertex_buffer(&mut self, handle : &VertexBufferHandle);
    fn unbind_vertex_buffer(&mut self, handle : &VertexBufferHandle);
    fn bind_vertex_array(&mut self, handle : &VertexArrayHandle);
    fn unbind_vertex_array(&mut self, handle : &VertexArrayHandle);
    fn bind_index_buffer(&mut self, handle : &IndexBufferHandle);
    fn unbind_index_buffer(&mut self, handle : &IndexBufferHandle);
    fn bind_shader(&mut self, handle : &ShaderHandle);
    fn unbind_shader(&mut self, handle : &ShaderHandle);

    // Operations: Index buffer 
    fn get_index_buffer_count(&self, handle : &IndexBufferHandle);

    // Operations: Vertex Buffer
    fn get_vertex_buffer_layout(&self, handle : &VertexBufferHandle) -> &BufferLayout;
    fn set_vertex_buffer_layout(&self, handle : &VertexBufferHandle, layout : BufferLayout);
      
    // Operations: Vertex Array
    fn set_vertex_array_vertex_buffer(&mut self, va_handle : &VertexArrayHandle, vb_handle : &VertexBufferHandle);
    fn set_vertex_array_index_buffer(&mut self, va_handle : &VertexArrayHandle, ib_handle : &IndexBufferHandle);
    fn get_vertex_array_vertex_buffer(&self, va_handle : &VertexArrayHandle) -> Option<VertexBufferHandle>;
    fn get_vertex_array_index_buffer(&self, va_handle : &VertexArrayHandle) -> Option<IndexBufferHandle>;

    // Operations: Shaders
    fn get_shader_name(&self, handle : &ShaderHandle) -> &str;
    fn set_shader_uniform_f32(&mut self, name : &str, value : f32);
    fn set_shader_uniform_i32(&mut self, name : &str, value : i32);
    fn set_shader_uniform_fvec2(&mut self, name: &str, value: &glam::Vec2);
    fn set_shader_uniform_fvec3(&mut self, name: &str, value: &glam::Vec3);
    fn set_shader_uniform_fvec4(&mut self, name: &str, value: &glam::Vec4);
    fn set_shader_uniform_fmat3(&mut self, name: &str, value: &glam::Mat3);
    fn set_shader_uniform_fmat4(&mut self, name: &str, value: &glam::Mat4);
    fn add_shader_uniform(&mut self, name : &str, data_type : ShaderDataType);
}

/// Implement this trait to support a new Render API
pub trait RenderAPIBackend: RenderAPIBackendDyn {
    fn create() -> RenderAPIBackendPtr;
}

pub type RenderAPIBackendPtr = Box<dyn RenderAPIBackendDyn>;

lazy_static! {
    static ref RENDER_API: RwLock<RenderCommand> = RwLock::new(RenderCommand { backend: None });
}

/// RenderCommand is a class we use to interface with the currently used backend
pub struct RenderCommand {
    backend: Option<RenderAPIBackendPtr>,
}

impl RenderCommand {
    pub fn initialize(platform: Platforms) {
        let mut render_api = RENDER_API.write();
        assert!(
            render_api.backend.is_none(),
            "Render api already initialized"
        );
        match platform {
            Platforms::Windows => {
                render_api.backend = Some(OpenGLRenderBackend::create());
            }
            _ => panic!("Platform Render API backend not yet implemented"),
        }
    }

    fn get_backend(&self) -> &RenderAPIBackendPtr {
        debug_assert!(self.backend.is_some(), "render api not initialized!");
        self.backend.as_ref().unwrap()
    }

    fn get_backend_mut(&mut self) -> &mut RenderAPIBackendPtr {
        debug_assert!(self.backend.is_some(), "render api not initialized!");
        self.backend.as_mut().unwrap()
    }

    pub fn draw_indexed(vertex_array: &dyn VertexArrayDyn) {
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

    pub fn set_clear_color(color: Color) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_clear_color(color);
    }

    pub fn set_viewport(x: u32, y: u32, width: u32, height: u32) {
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
