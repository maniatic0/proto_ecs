use glow::{self, NativeUniformLocation};
pub use proto_ecs::core::rendering::shader::ShaderError;
use std::collections::HashMap;

use crate::core::rendering::shader::ShaderDataType;

pub(super) struct OpenGLShader {
    pub(super) name: String,
    pub(super) native_program: glow::NativeProgram,
    pub(super) uniforms: HashMap<String, UniformData>,
}

// TODO Actual Send + Sync implementation
unsafe impl Send for OpenGLShader {}
unsafe impl Sync for OpenGLShader {}

pub(super) struct UniformData {
    pub(super) data_type: ShaderDataType,
    pub(super) location: NativeUniformLocation,
}

