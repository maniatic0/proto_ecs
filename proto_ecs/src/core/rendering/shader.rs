

/// Possible uniform data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct ShaderDataType {
    pub precision : Precision,
    pub data_type : DataType
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    P64,
    P32,
    P16, 
    P8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    None,

    Float,
    Float2,
    Float3,
    Float4,

    Int,
    Int2,
    Int3,
    Int4,

    Mat3,
    Mat4,

    Bool
}

impl ShaderDataType {
    pub fn new(precision : Precision, data_type : DataType) -> Self {
        Self{precision, data_type}
    }
    /// Size in bytes for this data type
    pub fn get_size(&self) -> u32 {
        let ShaderDataType { precision, data_type }  = self;
        match (data_type, precision) {
            (DataType::None, _) => 0,
            (DataType::Float | DataType::Int, Precision::P32) => 4,
            (DataType::Float2 | DataType::Int2, Precision::P32) => 2 * 4,
            (DataType::Float3 | DataType::Int3, Precision::P32) => 3 * 4,
            (DataType::Float4 | DataType::Int4, Precision::P32) => 4 * 4,

            (DataType::Float | DataType::Int, Precision::P16) => 2,
            (DataType::Float2 | DataType::Int2, Precision::P16) => 2 * 2,
            (DataType::Float3 | DataType::Int3, Precision::P16) => 3 * 2,
            (DataType::Float4 | DataType::Int4, Precision::P16) => 4 * 2,

            (DataType::Int, Precision::P8) => 1,
            (DataType::Int2, Precision::P8) => 2 * 1,
            (DataType::Int3, Precision::P8) => 3 * 1,
            (DataType::Int4, Precision::P8) => 4 * 1,

            (DataType::Mat3, Precision::P32) => 3 * 3 * 4,
            (DataType::Mat4, Precision::P32) => 4 * 4 * 4,
            (DataType::Bool, Precision::P8) => 1,

            _ => unimplemented!("This type/precision is not currently supported")
        }
    }
}

#[derive(Debug)]
pub enum ShaderError {
    /// Could not compile this shader
    CompilationError(String),
    /// Colliding name of the uniform, and already registered type
    UniformAlreadyExists {
        uniform_name: String,
        prev_type: ShaderDataType,
    },
    /// Trying to assing data to a uniform with the wrong data type
    InvalidTypeForUniform {
        uniform_name: String,
        expected_type: ShaderDataType,
        given_type: ShaderDataType,
    },
}

pub enum ShaderSrc<'a> {
    Binary(&'a [u8]),
    Code(&'a str),
}

// TODO Some types from [ShaderDataType] are missing here because glam does not support them. Even f16 is nightly in Rust. 
// What should we do about those types? 
#[derive(Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
pub enum ShaderDataTypeValue {

    Float_32(f32),
    Float2_32(glam::Vec2),
    Float3_32(glam::Vec3),
    Float4_32(glam::Vec4),

    Mat3_32(glam::Mat3),
    Mat4_32(glam::Mat4),

    Int_16(i16),
    Int2_16(glam::I16Vec2),
    Int3_16(glam::I16Vec3),
    Int4_16(glam::I16Vec4),

    Int_32(i32),
    Int2_32(glam::IVec2),
    Int3_32(glam::IVec3),
    Int4_32(glam::IVec4),

    Bool(bool),
}