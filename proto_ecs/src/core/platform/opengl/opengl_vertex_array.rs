use glow::HasContext;
use glow::NativeVertexArray;
use proto_ecs::core::platform::opengl::opengl_buffer::OpenGLVertexBuffer;
use proto_ecs::core::platform::opengl::opengl_render_backend::get_context;
use proto_ecs::core::rendering::buffer::{IndexBufferPtr, VertexBufferDyn, VertexBufferPtr};
use proto_ecs::core::rendering::vertex_array::{VertexArray, VertexArrayDyn};

use crate::core::rendering::shader::ShaderDataType;
use crate::core::rendering::vertex_array::VertexArrayPtr;

pub struct OpenGLVertexArray {
    native_array: NativeVertexArray,
    vertex_buffer: Option<VertexBufferPtr>,
    index_buffer: Option<IndexBufferPtr>,
}

impl VertexArrayDyn for OpenGLVertexArray {
    fn set_vertex_buffer(
        &mut self,
        vertex_buffer: crate::core::rendering::buffer::VertexBufferPtr,
    ) {
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
    fn set_index_buffer(&mut self, index_buffer: IndexBufferPtr) {
        self.index_buffer = Some(index_buffer);
    }

    fn bind(&self) {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.bind_vertex_array(Some(self.native_array));
        }
        if let Some(ib) = self.index_buffer.as_ref() {
            ib.bind();
        }
        if let Some(vb) = self.vertex_buffer.as_ref() {
            vb.bind();
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
    fn create() -> VertexArrayPtr {
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
