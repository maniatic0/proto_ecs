/// Possible uniform data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderDataType {
    None,
    Float,
    Float2,
    Float3,
    Float4,
    Mat3,
    Mat4,
    Int,
    Int2,
    Int3,
    Int4,
    Bool,
}

impl ShaderDataType {
    /// Size in bytes for this data type
    pub fn get_size(&self) -> u32 {
        match self {
            ShaderDataType::None => 0,
            ShaderDataType::Float | ShaderDataType::Int => 4,
            ShaderDataType::Float2 | ShaderDataType::Int2 => 2 * 4,
            ShaderDataType::Float3 | ShaderDataType::Int3 => 3 * 4,
            ShaderDataType::Float4 | ShaderDataType::Int4 => 4 * 4,
            ShaderDataType::Mat3 => 3 * 3 * 4,
            ShaderDataType::Mat4 => 4 * 4 * 4,
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
