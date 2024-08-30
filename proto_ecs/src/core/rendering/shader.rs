/// Possible uniform data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ShaderDataType {
    None,

    Float_16,
    Float2_16,
    Float3_16,
    Float4_16,

    Float_32,
    Float2_32,
    Float3_32,
    Float4_32,

    Mat3_32,
    Mat4_32,

    Int_8,
    Int2_8,
    Int3_8,
    Int4_8,

    Int_16,
    Int2_16,
    Int3_16,
    Int4_16,

    Int_32,
    Int2_32,
    Int3_32,
    Int4_32,

    Bool,
}

impl ShaderDataType {
    /// Size in bytes for this data type
    pub fn get_size(&self) -> u32 {
        match self {
            ShaderDataType::None => 0,
            ShaderDataType::Float_32 | ShaderDataType::Int_32 => 4,
            ShaderDataType::Float2_32 | ShaderDataType::Int2_32 => 2 * 4,
            ShaderDataType::Float3_32 | ShaderDataType::Int3_32 => 3 * 4,
            ShaderDataType::Float4_32 | ShaderDataType::Int4_32 => 4 * 4,

            ShaderDataType::Float_16 | ShaderDataType::Int_16 => 2,
            ShaderDataType::Float2_16 | ShaderDataType::Int2_16 => 2 * 2,
            ShaderDataType::Float3_16 | ShaderDataType::Int3_16 => 3 * 2,
            ShaderDataType::Float4_16 | ShaderDataType::Int4_16 => 4 * 2,

            ShaderDataType::Int_8 => 1,
            ShaderDataType::Int2_8 => 2 * 1,
            ShaderDataType::Int3_8 => 3 * 1,
            ShaderDataType::Int4_8 => 4 * 1,

            ShaderDataType::Mat3_32 => 3 * 3 * 4,
            ShaderDataType::Mat4_32 => 4 * 4 * 4,
            ShaderDataType::Bool => 1,
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
