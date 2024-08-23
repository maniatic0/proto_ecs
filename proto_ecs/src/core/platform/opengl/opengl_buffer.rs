use ecs_macros::CanCast;
use glow::NativeBuffer;
use proto_ecs::core::rendering::buffer::BufferLayout;

#[derive(CanCast)]
pub struct OpenGLIndexBuffer {
    pub(super) native_buffer: NativeBuffer,
    pub(super) element_count: usize,
}

#[derive(CanCast)]
pub struct OpenGLVertexBuffer {
    pub(super) native_buffer: NativeBuffer,
    pub(super) buffer_layout: BufferLayout,
}

impl OpenGLVertexBuffer {
    #[inline(always)]
    pub(super) fn get_buffer_layout(&self) -> &BufferLayout {
        &self.buffer_layout
    }
}
