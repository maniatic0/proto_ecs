use glow::NativeVertexArray;

use crate::core::rendering::render_api::IndexBufferHandle;
use crate::core::rendering::render_api::VertexBufferHandle;

pub struct OpenGLVertexArray {
    pub(super) native_array: NativeVertexArray,
    pub(super) vertex_buffer: Option<VertexBufferHandle>,
    pub(super) index_buffer: Option<IndexBufferHandle>,
}
// TODO Actual Send + Sync implementation
unsafe impl Send for OpenGLVertexArray {}
unsafe impl Sync for OpenGLVertexArray {}
