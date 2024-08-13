use super::{render_api::API, Render};
use proto_ecs::core::math::glam;
use proto_ecs::core::platform::opengl::opengl_shader::OpenGLShader;

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
pub trait ShaderDyn {
    fn bind(&self);
    fn unbind(&self);
    fn get_name(&self) -> &String;

    fn set_uniform_f32(&self, name: &str, value: f32);
    fn set_uniform_i32(&self, name: &str, value: i32);
    fn set_uniform_fvec2(&self, name: &str, value: &glam::Vec2);
    fn set_uniform_fvec3(&self, name: &str, value: &glam::Vec3);
    fn set_uniform_fvec4(&self, name: &str, value: &glam::Vec4);
    fn set_uniform_fmat3(&self, name: &str, value: &glam::Mat3);
    fn set_uniform_fmat4(&self, name: &str, value: &glam::Mat4);

    fn add_uniform(&mut self, name: &str, data_type: ShaderDataType) -> Result<(), ShaderError>;
}

pub type ShaderPtr = Box<dyn ShaderDyn>;

/// Implement this trait for a specific platform to provide support for it
pub trait Shader: ShaderDyn {
    fn create(
        name: &str,
        vertex_source: &str,
        fragment_source: &str,
    ) -> Result<ShaderPtr, ShaderError>;

    fn create_from_file(name: &str) -> Result<ShaderPtr, ShaderError>;
}

pub fn create_shader(
    name: &str,
    vertex_source: &str,
    fragment_source: &str,
) -> Result<ShaderPtr, ShaderError> {
    match Render::get_current_api() {
        API::OpenGL => OpenGLShader::create(name, vertex_source, fragment_source),
        _ => unimplemented!("API not yet implemented"),
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
