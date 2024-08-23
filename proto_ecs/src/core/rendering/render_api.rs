use super::buffer::BufferLayout;
use super::shader::ShaderDataType;
use lazy_static::lazy_static;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::math::Color;
use proto_ecs::core::platform::opengl::opengl_render_backend::OpenGLRenderBackend;
use proto_ecs::core::platform::Platforms;
use proto_ecs::core::rendering::handle::Handle;
use proto_ecs::core::rendering::shader::ShaderError;

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
    fn draw_indexed(&mut self, handle: VertexArrayHandle);

    // Resource creation and destruction
    fn create_vertex_buffer(&mut self, vertex_data: &[f32]) -> VertexBufferHandle;
    fn destroy_vertex_buffer(&mut self, handle: VertexBufferHandle);
    fn create_index_buffer(&mut self, indices: &[u32]) -> IndexBufferHandle;
    fn destroy_index_buffer(&mut self, handle: IndexBufferHandle);
    fn create_vertex_array(&mut self) -> VertexArrayHandle;
    fn destroy_vertex_array(&mut self, handle: VertexArrayHandle);
    fn create_shader(
        &mut self,
        name: &str,
        vertex_src: &str,
        fragment_src: &str,
    ) -> Result<ShaderHandle, ShaderError>;
    fn destroy_shader(&mut self, handle: ShaderHandle);

    // Bindings
    fn bind_vertex_buffer(&self, handle: VertexBufferHandle);
    fn unbind_vertex_buffer(&self);
    fn bind_vertex_array(&self, handle: VertexArrayHandle);
    fn unbind_vertex_array(&self);
    fn bind_index_buffer(&self, handle: IndexBufferHandle);
    fn unbind_index_buffer(&self);
    fn bind_shader(&self, handle: ShaderHandle);
    fn unbind_shader(&self);

    // Operations: Index buffer
    fn get_index_buffer_count(&self, handle: IndexBufferHandle) -> u32;

    // Operations: Vertex Buffer
    fn get_vertex_buffer_layout(&self, handle: VertexBufferHandle) -> &BufferLayout;
    fn set_vertex_buffer_layout(&self, handle: VertexBufferHandle, layout: BufferLayout);

    // Operations: Vertex Array
    fn set_vertex_array_vertex_buffer(
        &mut self,
        va_handle: VertexArrayHandle,
        vb_handle: VertexBufferHandle,
    );
    fn set_vertex_array_index_buffer(
        &mut self,
        va_handle: VertexArrayHandle,
        ib_handle: IndexBufferHandle,
    );
    fn get_vertex_array_vertex_buffer(
        &self,
        va_handle: VertexArrayHandle,
    ) -> Option<VertexBufferHandle>;
    fn get_vertex_array_index_buffer(
        &self,
        va_handle: VertexArrayHandle,
    ) -> Option<IndexBufferHandle>;

    // Operations: Shaders
    fn get_shader_name(&self, handle: ShaderHandle) -> &str;
    fn set_shader_uniform_f32(&mut self, handle: ShaderHandle, name: &str, value: f32);
    fn set_shader_uniform_i32(&mut self, handle: ShaderHandle, name: &str, value: i32);
    fn set_shader_uniform_fvec2(&mut self, handle: ShaderHandle, name: &str, value: &glam::Vec2);
    fn set_shader_uniform_fvec3(&mut self, handle: ShaderHandle, name: &str, value: &glam::Vec3);
    fn set_shader_uniform_fvec4(&mut self, handle: ShaderHandle, name: &str, value: &glam::Vec4);
    fn set_shader_uniform_fmat3(&mut self, handle: ShaderHandle, name: &str, value: &glam::Mat3);
    fn set_shader_uniform_fmat4(&mut self, handle: ShaderHandle, name: &str, value: &glam::Mat4);
    fn add_shader_uniform(
        &mut self,
        handle: ShaderHandle,
        name: &str,
        data_type: ShaderDataType,
    ) -> Result<(), ShaderError>;
}

/// Implement this trait to support a new Render API
pub trait RenderAPIBackend: RenderAPIBackendDyn {
    fn create() -> RenderAPIBackendPtr;
}

pub type RenderAPIBackendPtr = Box<dyn RenderAPIBackendDyn>;

lazy_static! {
    static ref RENDER_API: RwLock<RenderCommand> = RwLock::new(RenderCommand { backend: None });
}

/// RenderCommand is a class we use to interface with the currently used backend.
/// It stores the backend object and additional necessary metadata or state data.
///
/// There's usually a single instance of this class (a singleton) that you interact  
/// with using static methods.
///
/// The point of this class is to control how the render api backend is accessed, including
/// locking methods
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

    #[inline(always)]
    fn get_backend(&self) -> &RenderAPIBackendPtr {
        debug_assert!(self.backend.is_some(), "render api not initialized!");
        self.backend.as_ref().unwrap()
    }

    #[inline(always)]
    fn get_backend_mut(&mut self) -> &mut RenderAPIBackendPtr {
        debug_assert!(self.backend.is_some(), "render api not initialized!");
        self.backend.as_mut().unwrap()
    }

    pub fn draw_indexed(handle: VertexArrayHandle) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.draw_indexed(handle);
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

    // -- < Methods that come from the render api trait > -------------------------------------
    // Resource creation and destruction
    pub fn create_vertex_buffer(vertex_data: &[f32]) -> VertexBufferHandle {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.create_vertex_buffer(vertex_data)
    }

    pub fn destroy_vertex_buffer(handle: VertexBufferHandle) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.destroy_vertex_buffer(handle)
    }
    pub fn create_index_buffer(indices: &[u32]) -> IndexBufferHandle {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.create_index_buffer(indices)
    }
    pub fn destroy_index_buffer(handle: IndexBufferHandle) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.destroy_index_buffer(handle)
    }
    pub fn create_vertex_array() -> VertexArrayHandle {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.create_vertex_array()
    }
    pub fn destroy_vertex_array(handle: VertexArrayHandle) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.destroy_vertex_array(handle)
    }
    pub fn create_shader(
        name: &str,
        vertex_src: &str,
        fragment_src: &str,
    ) -> Result<ShaderHandle, ShaderError> {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.create_shader(name, vertex_src, fragment_src)
    }
    pub fn destroy_shader(handle: ShaderHandle) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.destroy_shader(handle)
    }

    // Bindings
    pub fn bind_vertex_buffer(handle: VertexBufferHandle) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.bind_vertex_buffer(handle)
    }
    pub fn unbind_vertex_buffer() {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.unbind_vertex_buffer()
    }
    pub fn bind_vertex_array(handle: VertexArrayHandle) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.bind_vertex_array(handle)
    }
    pub fn unbind_vertex_array() {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.unbind_vertex_array()
    }
    pub fn bind_index_buffer(handle: IndexBufferHandle) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.bind_index_buffer(handle)
    }
    pub fn unbind_index_buffer() {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.unbind_index_buffer()
    }
    pub fn bind_shader(handle: ShaderHandle) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.bind_shader(handle)
    }
    pub fn unbind_shader() {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.unbind_shader()
    }

    // Operations: Index buffer
    pub fn get_index_buffer_count(handle: IndexBufferHandle) -> u32 {
        let api = RENDER_API.read();
        let backend = api.get_backend();
        backend.get_index_buffer_count(handle)
    }

    // Operations: Vertex Buffer
    pub fn get_vertex_buffer_layout(handle: VertexBufferHandle) -> BufferLayout {
        let api = RENDER_API.read();
        let backend = api.get_backend();
        let buffer_layout = backend.get_vertex_buffer_layout(handle);
        buffer_layout.clone()
    }
    pub fn set_vertex_buffer_layout(handle: VertexBufferHandle, layout: BufferLayout) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_vertex_buffer_layout(handle, layout)
    }

    // Operations: Vertex Array
    pub fn set_vertex_array_vertex_buffer(
        va_handle: VertexArrayHandle,
        vb_handle: VertexBufferHandle,
    ) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_vertex_array_vertex_buffer(va_handle, vb_handle)
    }
    pub fn set_vertex_array_index_buffer(
        va_handle: VertexArrayHandle,
        ib_handle: IndexBufferHandle,
    ) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_vertex_array_index_buffer(va_handle, ib_handle)
    }
    pub fn get_vertex_array_vertex_buffer(
        va_handle: VertexArrayHandle,
    ) -> Option<VertexBufferHandle> {
        let api = RENDER_API.read();
        let backend = api.get_backend();
        backend.get_vertex_array_vertex_buffer(va_handle)
    }
    pub fn get_vertex_array_index_buffer(
        va_handle: VertexArrayHandle,
    ) -> Option<IndexBufferHandle> {
        let api = RENDER_API.read();
        let backend = api.get_backend();
        backend.get_vertex_array_index_buffer(va_handle)
    }

    // Operations: Shaders
    pub fn get_shader_name(handle: ShaderHandle) -> String {
        let api = RENDER_API.read();
        let backend = api.get_backend();
        backend.get_shader_name(handle).to_string()
    }
    pub fn set_shader_uniform_f32(handle: ShaderHandle, name: &str, value: f32) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_shader_uniform_f32(handle, name, value)
    }
    pub fn set_shader_uniform_i32(handle: ShaderHandle, name: &str, value: i32) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_shader_uniform_i32(handle, name, value)
    }
    pub fn set_shader_uniform_fvec2(handle: ShaderHandle, name: &str, value: &glam::Vec2) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_shader_uniform_fvec2(handle, name, value)
    }
    pub fn set_shader_uniform_fvec3(handle: ShaderHandle, name: &str, value: &glam::Vec3) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_shader_uniform_fvec3(handle, name, value)
    }
    pub fn set_shader_uniform_fvec4(handle: ShaderHandle, name: &str, value: &glam::Vec4) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_shader_uniform_fvec4(handle, name, value)
    }
    pub fn set_shader_uniform_fmat3(handle: ShaderHandle, name: &str, value: &glam::Mat3) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_shader_uniform_fmat3(handle, name, value)
    }
    pub fn set_shader_uniform_fmat4(handle: ShaderHandle, name: &str, value: &glam::Mat4) {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.set_shader_uniform_fmat4(handle, name, value)
    }
    pub fn add_shader_uniform(
        handle: ShaderHandle,
        name: &str,
        data_type: ShaderDataType,
    ) -> Result<(), ShaderError> {
        let mut api = RENDER_API.write();
        let backend = api.get_backend_mut();
        backend.add_shader_uniform(handle, name, data_type)
    }
}
