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
use crate::core::render::buffer::{VertexArray, VertexArrayDyn};
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

pub struct OpenGLVertexArray {
    native_array: NativeVertexArray,
    vertex_buffer: Option<VertexBufferPtr>,
    index_buffer: Option<IndexBufferPtr>,
}

impl VertexArrayDyn for OpenGLVertexArray {
    fn set_vertex_buffer(&mut self, vertex_buffer: crate::core::render::buffer::VertexBufferPtr) {
        let opengl_buffer = vertex_buffer
            .into_any()
            .downcast::<OpenGLVertexBuffer>()
            .expect("Incompatible vertex buffer");

        self.bind();
        opengl_buffer.bind();
        let layout = opengl_buffer.get_layout();
        {
            get_context!(context);
            let gl = &context.gl;
            for (i, element) in layout.iter().enumerate() {
                unsafe {
                    gl.enable_vertex_attrib_array(i as u32);
                    let element_count = element.get_component_count();
                    match element.get_data_type() {
                        ShaderDataType::Float
                        | ShaderDataType::Float2
                        | ShaderDataType::Float3
                        | ShaderDataType::Float4
                        | ShaderDataType::Mat3
                        | ShaderDataType::Mat4 => {
                            gl.vertex_attrib_pointer_f32(
                                i as u32,
                                element_count as i32,
                                glow::FLOAT,
                                element.is_normalized(),
                                layout.get_stride() as i32,
                                element.get_offset() as i32,
                            );
                        }
                        ShaderDataType::Int
                        | ShaderDataType::Int2
                        | ShaderDataType::Int3
                        | ShaderDataType::Int4
                        | ShaderDataType::Bool => gl.vertex_attrib_pointer_i32(
                            i as u32,
                            element_count as i32,
                            glow::INT,
                            layout.get_stride() as i32,
                            element.get_offset() as i32,
                        ),
                        _ => panic!("Don't know how define attribute of this type"),
                    }
                }
            }
        }
        opengl_buffer.unbind();
        self.vertex_buffer = Some(opengl_buffer);
    }

    fn get_index_buffer(&self) -> &Option<IndexBufferPtr> {
        &self.index_buffer
    }

    fn get_vertex_buffer(&self) -> &Option<VertexBufferPtr> {
        &self.vertex_buffer
    }
    fn set_index_buffer(&mut self, index_buffer: crate::core::render::buffer::IndexBufferPtr) {
        self.index_buffer = Some(index_buffer);
    }

    fn bind(&self) {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.bind_vertex_array(Some(self.native_array));
        }
    }

    fn unbind(&self) {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.bind_vertex_array(None);
        }
    }
}

impl VertexArray for OpenGLVertexArray {
    fn create() -> crate::core::render::buffer::VertexArrayPtr {
        get_context!(context);
        let gl = &context.gl;
        let native_array = unsafe {
            gl.create_vertex_array()
                .expect("Could not create OpenGL vertex array")
        };

        Box::new(OpenGLVertexArray {
            native_array,
            vertex_buffer: None,
            index_buffer: None,
        })
    }
}
