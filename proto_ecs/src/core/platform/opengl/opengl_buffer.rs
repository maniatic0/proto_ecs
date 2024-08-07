use ecs_macros::CanCast;
use glow::HasContext;
use glow::NativeBuffer;
use glow::NativeVertexArray;
use proto_ecs::core::platform::opengl::opengl_render_backend::get_context;
use proto_ecs::core::render::buffer::{
    BufferLayout, IndexBuffer, IndexBufferDyn, VertexBuffer, VertexBufferDyn,
};

use crate::core::render::buffer::IndexBufferPtr;
use crate::core::render::buffer::VertexBufferPtr;
use crate::core::render::vertex_array::{VertexArrayPtr, VertexArray, VertexArrayDyn};
use crate::core::render::shader::ShaderDataType;

#[derive(CanCast)]
pub struct OpenGLIndexBuffer {
    native_buffer: NativeBuffer,
    element_count: usize,
}

#[derive(CanCast)]
pub struct OpenGLVertexBuffer {
    native_buffer: NativeBuffer,
    buffer_layout: BufferLayout,
}

impl IndexBuffer for OpenGLIndexBuffer {
    fn create(indices: &[u32]) -> crate::core::render::buffer::IndexBufferPtr {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            // TODO Better error handling would be nice
            let buffer_id = gl.create_buffer().expect("Unable to create index buffer");

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(buffer_id));
            let u8_slice = std::mem::transmute(indices);
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, u8_slice, glow::STATIC_DRAW);
            return Box::new(OpenGLIndexBuffer {
                native_buffer: buffer_id,
                element_count: indices.len(),
            });
        }
    }
}

impl IndexBufferDyn for OpenGLIndexBuffer {
    fn bind(&self) {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.native_buffer));
        }
    }

    fn get_count(&self) -> u32 {
        self.element_count as u32
    }

    fn unbind(&self) {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        }
    }
}

impl VertexBuffer for OpenGLVertexBuffer {
    fn create(vertices: &[f32]) -> crate::core::render::buffer::VertexBufferPtr {
        get_context!(context);
        let gl = &context.gl;

        unsafe {
            // TODO Better error handling
            let native_buffer = gl.create_buffer().expect("Could not create vertex buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(native_buffer));
            let bytes: &[u8] = std::mem::transmute(vertices);
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, &bytes, glow::STATIC_DRAW);

            return Box::new(OpenGLVertexBuffer {
                native_buffer,
                buffer_layout: BufferLayout::default(),
            });
        }
    }
}

impl VertexBufferDyn for OpenGLVertexBuffer {
    fn bind(&self) {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.native_buffer));
        }
    }

    fn get_layout(&self) -> &crate::core::render::buffer::BufferLayout {
        &self.buffer_layout
    }

    fn set_layout(&mut self, new_layout: crate::core::render::buffer::BufferLayout) {
        self.buffer_layout = new_layout;
    }

    fn unbind(&self) {
        get_context!(context);
        let gl = &context.gl;

        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }
}
