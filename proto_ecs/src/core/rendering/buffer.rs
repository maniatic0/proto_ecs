use proto_ecs::core::rendering::shader::ShaderDataType;
use std::slice::{Iter, IterMut};

#[derive(Default, Clone)]
pub struct BufferLayout {
    elements: Vec<BufferElement>,
    stride: u32,
}

impl BufferLayout {
    pub fn from_elements(elements: Vec<BufferElement>) -> Self {
        let mut layout = BufferLayout {
            elements,
            stride: 0,
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
#[derive(Debug, Clone)]
pub struct BufferElement {
    name: String,
    data_type: ShaderDataType,
    size: u32,
    offset: u32,
    normalized: bool,
}

impl BufferElement {
    pub fn new(name: String, data_type: ShaderDataType, normalized: bool) -> Self {
        BufferElement {
            size: data_type.get_size(),
            name,
            data_type,
            normalized,
            offset: 0,
        }
    }

    pub fn get_component_count(&self) -> u32 {
        match self.data_type {
            ShaderDataType::Float_32
            | ShaderDataType::Float_16
            | ShaderDataType::Bool
            | ShaderDataType::Int_32
            | ShaderDataType::Int_16
            | ShaderDataType::Int_8 => 1,

            ShaderDataType::Float2_32
            | ShaderDataType::Float2_16
            | ShaderDataType::Int2_32
            | ShaderDataType::Int2_16
            | ShaderDataType::Int2_8 => 2,

            ShaderDataType::Float3_32
            | ShaderDataType::Float3_16
            | ShaderDataType::Int3_32
            | ShaderDataType::Int3_16
            | ShaderDataType::Int3_8 => 3,

            ShaderDataType::Float4_32 | ShaderDataType::Float4_16 | ShaderDataType::Int4_32 | ShaderDataType::Int4_16 | ShaderDataType::Int4_8=> 4,
            ShaderDataType::Mat3_32 => 3 * 3,
            ShaderDataType::Mat4_32 => 4 * 4,
            ShaderDataType::None => 0,
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

    #[inline(always)]
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }
}
