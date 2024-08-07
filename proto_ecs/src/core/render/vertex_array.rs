use proto_ecs::core::render::Render;
use proto_ecs::core::render::render_api::API;
use proto_ecs::core::render::buffer::{VertexBufferPtr, IndexBufferPtr};
use crate::core::platform::opengl::opengl_vertex_array::OpenGLVertexArray;

pub trait VertexArrayDyn {
    fn bind(&self);
    fn unbind(&self);
    fn set_vertex_buffer(&mut self, vertex_buffer : VertexBufferPtr);
    fn set_index_buffer(&mut self, index_buffer : IndexBufferPtr);
    fn get_vertex_buffer(&self) -> &Option<VertexBufferPtr>;
    fn get_index_buffer(&self) -> &Option<IndexBufferPtr>;
}
pub trait VertexArray : VertexArrayDyn {
    fn create() -> VertexArrayPtr;
}

pub type VertexArrayPtr = Box<dyn VertexArrayDyn>;

pub fn create_vertex_array() -> VertexArrayPtr {
    match Render::get_current_api() {
        API::OpenGL => OpenGLVertexArray::create(),
        _ => unimplemented!("Creation of vertex array not yet implemented for the current API")
    }
}