use crate::core::utils::handle::{Allocator, Handle};

use super::{render_api::ShaderHandle, shader::ShaderDataTypeValue};
use std::collections::HashMap;

type MaterialArguments = HashMap<String, ShaderDataTypeValue>;

#[derive(Debug)]
pub struct Material {
    pub(crate) shader: ShaderHandle,
    parameters: MaterialArguments,
}

impl Material {

    /// Set a parameter for the shader in this material. The existence of the parameter 
    /// is not checked in this function, but when this material gets actually used in a shader
    pub fn set_parameter(&mut self, parameter: &str, value: ShaderDataTypeValue) {
        self.parameters
            .entry(parameter.into())
            .and_modify(|old_value| *old_value = value.clone())
            .or_insert(value);
    }
}

pub type MaterialAllocator = Allocator<Material>;
pub type MaterialHandle = Handle;