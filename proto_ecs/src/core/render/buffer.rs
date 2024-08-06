use std::slice::{Iter, IterMut};
use proto_ecs::core::render::shader::ShaderDataType;

use crate::core::{casting::CanCast, platform::opengl::opengl_buffer::{OpenGLIndexBuffer, OpenGLVertexBuffer, OpenGLVertexArray}};

use super::{render_api::API, Render};

pub trait VertexBufferDyn : CanCast{
    fn bind(&self);
    fn unbind(&self);
    fn get_layout(&self) -> &BufferLayout; 
    fn set_layout(&mut self, new_layout : BufferLayout); 
}

/// Implement this trait to create a platform-specific Vertex Buffer
pub trait VertexBuffer : VertexBufferDyn  {
    fn create(vertices : &[f32]) -> VertexBufferPtr;
}

pub fn create_vertex_buffer (vertices : &[f32]) -> VertexBufferPtr {
    let api = Render::get_current_api();
    match api {
        API::OpenGL => OpenGLVertexBuffer::create(vertices),
        _ => unimplemented!("Vertex buffer not yet implemented for this graphics API")
    }
}

pub type VertexBufferPtr = Box<dyn VertexBufferDyn>;

pub trait IndexBufferDyn {
    fn bind(&self);
    fn unbind(&self);
    fn get_count(&self) -> u32;
}

/// Implement this trait to create a platform-specific Index Buffer
pub trait IndexBuffer : IndexBufferDyn {
    fn create(indices : &[u32]) -> IndexBufferPtr;
}

pub type IndexBufferPtr = Box<dyn IndexBufferDyn>;

pub fn create_index_buffer(indices : &[u32]) -> IndexBufferPtr {
    match Render::get_current_api() {
        API::OpenGL => OpenGLIndexBuffer::create(indices),
        _ => unimplemented!("Render API not yet implemented")
    }
}

#[derive(Default)]
pub struct BufferLayout {
    elements : Vec<BufferElement>,
    stride : u32
}

impl BufferLayout {
    pub fn from_elements(elements : Vec<BufferElement>) -> Self {
        let mut layout = BufferLayout {
            elements, 
            stride : 0
        };

        layout.compute_offset_and_stride();

        layout
    }
    fn compute_offset_and_stride(&mut self) {
        let mut offset = 0;
        for element in self.elements.iter_mut() {
            element.offset = offset;
            offset += element.size;
        }

        self.stride = offset;
    }

    #[inline(always)]
    pub fn get_buffer_elements(&self) -> &Vec<BufferElement> {
        &self.elements
    }

    #[inline(always)]
    pub fn get_stride(&self) -> u32 {
        self.stride
    }

    #[inline(always)]
    pub fn iter(&self) -> Iter<'_, BufferElement> {
        self.elements.iter()
    }

    #[inline(always)]
    pub fn iter_mut(&mut self) -> IterMut<'_, BufferElement> {
        self.elements.iter_mut()
    }
}

/// Describes a buffer element, part of the vertex data to send to a shader
pub struct BufferElement {
    name : String,
    data_type : ShaderDataType,
    size : u32, 
    offset : u32,
    normalized : bool
}

impl BufferElement {
    pub fn new(name : String, data_type : ShaderDataType, normalized : bool) -> Self {
        BufferElement {
            size : data_type.get_size(),
            name,
            data_type, 
            normalized,
            offset : 0
        }
    }

    pub fn get_component_count(&self) -> u32 {
        match self.data_type {
            ShaderDataType::Float | ShaderDataType::Bool | ShaderDataType::Int => 1,
            ShaderDataType::Float2 | ShaderDataType::Int2 => 2,
            ShaderDataType::Float3 | ShaderDataType::Int3 => 3,
            ShaderDataType::Float4 | ShaderDataType::Int4 => 4,
            ShaderDataType::Mat3 => 3*3,
            ShaderDataType::Mat4 => 4*4,
            ShaderDataType::None => 0
        }
    }

    pub fn get_data_type(&self) -> ShaderDataType {
        self.data_type
    }

    pub fn is_normalized(&self) -> bool {
        self.normalized
    }

    pub fn get_offset(&self) -> u32 {
        self.offset
    }
}

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