use proto_ecs::core::render::buffer::{VertexBufferPtr, IndexBufferPtr};

pub type VertexArrayPtr = Box<dyn VertexArrayDyn>;

pub trait VertexArrayDyn {
    fn bind(&self);
    fn unbind(&self);
    fn add_vertex_buffer(&mut self, vertex_buffer : &VertexBufferPtr);
    fn set_index_buffer(&mut self, index_buffer : &IndexBufferPtr);
    fn get_vertex_buffers(&self) -> Vec<&VertexBufferPtr>;
    fn get_index_buffer(&self) -> &IndexBufferPtr;
}

/// Implement this trait using platform-specific APIs to provide an abstract API
/// for the render
pub trait VertexArray : VertexArrayDyn {
    fn create() -> VertexArrayPtr;
}