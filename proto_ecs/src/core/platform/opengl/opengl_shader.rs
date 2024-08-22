use glow::{self, HasContext, NativeProgram, NativeShader, NativeUniformLocation};
use proto_ecs::core::platform::opengl::opengl_render_backend::get_context;
use proto_ecs::core::rendering::shader::ShaderError;
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

/// Compile shaders into a program. The vector of pairs goes from shader type (fragment, vertex)
/// to the shader code: (shader_type, shader_code)
pub(super) fn compile_shaders(shaders: Vec<(u32, &str)>) -> Result<NativeProgram, ShaderError> {
    get_context!(context);
    let gl = &context.gl;
    unsafe {
        let program = gl
            .create_program()
            .expect("Could not create program from OpenGL");
        let mut created_shaders: Vec<NativeShader> = vec![];

        for (shader_type, source) in shaders.iter() {
            let shader = gl
                .create_shader(*shader_type)
                .expect("Could not create OpenGL shader");
            gl.shader_source(shader, source);
            gl.compile_shader(shader);

            // Check if compilation for this shader went ok
            let is_compiled = gl.get_shader_compile_status(shader);
            if !is_compiled {
                let info_log = gl.get_shader_info_log(shader);

                // Delete previously created shaders
                gl.delete_shader(shader);
                for shader in created_shaders.into_iter() {
                    gl.delete_shader(shader)
                }

                // Delete program in progress
                gl.delete_program(program);

                eprintln!("Error creating shader: {}", info_log);
                return Err(ShaderError::CompilationError(info_log));
            }

            // Compilation ok, attach this shader to the program we are creating
            gl.attach_shader(program, shader);
            created_shaders.push(shader);
        }

        // Now that all shaders are compiled and attach to the program, we have to link the program
        gl.link_program(program);
        let is_linked = gl.get_program_link_status(program);
        if !is_linked {
            // If not ok, clean up all the resources we have created
            let info_log = gl.get_program_info_log(program);
            gl.delete_program(program);
            for shader in created_shaders.into_iter() {
                gl.delete_shader(shader);
            }

            eprintln!("Error linking program: {}", info_log);
            return Err(ShaderError::CompilationError(info_log));
        }

        // Program linking successfull: dettach shaders
        for shader in created_shaders.into_iter() {
            gl.detach_shader(program, shader);
        }

        Ok(program)
    }
}
