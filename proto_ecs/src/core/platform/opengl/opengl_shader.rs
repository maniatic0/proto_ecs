use glow::{self, HasContext, NativeProgram, NativeShader, NativeUniformLocation};
use proto_ecs::core::platform::opengl::opengl_render_backend::get_context;
use proto_ecs::core::render::shader::{Shader, ShaderDyn, ShaderError, ShaderPtr};
use std::collections::HashMap;

use crate::core::render::shader::ShaderDataType;

pub struct OpenGLShader {
    name: String,
    native_program: glow::NativeProgram,
    uniforms: HashMap<String, UniformData>,
}

struct UniformData {
    data_type: ShaderDataType,
    location: NativeUniformLocation,
}

impl Shader for OpenGLShader {
    fn create(
        name: &String,
        vertex_source: &String,
        fragment_source: &String,
    ) -> Result<ShaderPtr, ShaderError> {
        let mut shaders = Vec::<(u32, String)>::new();
        shaders.push((glow::VERTEX_SHADER, vertex_source.clone()));
        shaders.push((glow::FRAGMENT_SHADER, fragment_source.clone()));
        let uniforms = HashMap::new();

        let program = compile_shaders(shaders)?;
        Ok(Box::new(OpenGLShader {
            name: name.clone(),
            native_program: program,
            uniforms,
        }))
    }

    fn create_from_file(name: &String) -> Result<ShaderPtr, ShaderError> {
        unimplemented!("TODO")
    }
}

impl ShaderDyn for OpenGLShader {
    fn bind(&self) {
        get_context!(context);
        let gl = &context.gl;

        unsafe {
            gl.use_program(Some(self.native_program));
        }
    }

    fn unbind(&self) {
        get_context!(context);
        let gl = &context.gl;

        unsafe {
            gl.use_program(None);
        }
    }

    fn get_name(&self) -> &String {
        &self.name
    }

    fn add_uniform(
        &mut self,
        name: &String,
        data_type: crate::core::render::shader::ShaderDataType,
    ) -> Result<(), ShaderError> {
        if let Some(uniform_data) = self.uniforms.get(name) {
            return Err(ShaderError::UniformAlreadyExists {
                uniform_name: name.clone(),
                prev_type: uniform_data.data_type.clone(),
            });
        };
        get_context!(context);
        let gl = &context.gl;
        let location = unsafe {
            gl.get_uniform_location(self.native_program, name.as_str())
                .expect(
                    format!(
                        "Could not get an attribute location for shader '{}'",
                        self.name
                    )
                    .as_str(),
                )
        };
        self.uniforms.insert(
            name.clone(),
            UniformData {
                data_type,
                location,
            },
        );
        Ok(())
    }

    fn set_uniform_f32(&self, name: &String, value: f32) {
        let uniform_data = self
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Float,
            "Wrong uniform type"
        );
        get_context!(context);
        let gl = &context.gl;

        self.bind();
        unsafe {
            gl.uniform_1_f32(Some(&uniform_data.location), value);
        }
    }

    fn set_uniform_fmat3(&self, name: &String, value: glam::Mat3) {
        let uniform_data = self
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Mat3,
            "Wrong uniform type"
        );
        get_context!(context);
        let gl = &context.gl;

        self.bind();
        unsafe {
            gl.uniform_matrix_3_f32_slice(Some(&uniform_data.location),false, value.as_ref().as_slice());
        }
    }

    fn set_uniform_fmat4(&self, name: &String, value: glam::Mat4) {
        let uniform_data = self
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Mat4,
            "Wrong uniform type"
        );
        get_context!(context);
        let gl = &context.gl;

        self.bind();
        unsafe {
            gl.uniform_matrix_4_f32_slice(Some(&uniform_data.location),false, value.as_ref().as_slice());
        }
    }

    fn set_uniform_fvec2(&self, name: &String, value: glam::Vec2) {
        let uniform_data = self
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Float2,
            "Wrong uniform type"
        );
        get_context!(context);
        let gl = &context.gl;

        self.bind();
        unsafe {
            gl.uniform_2_f32(Some(&uniform_data.location), value.x, value.y);
        }
    }

    fn set_uniform_fvec3(&self, name: &String, value: glam::Vec3) {
        let uniform_data = self
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Float3,
            "Wrong uniform type"
        );
        get_context!(context);
        let gl = &context.gl;

        self.bind();
        unsafe {
            gl.uniform_3_f32(Some(&uniform_data.location), value.x, value.y, value.z);
        }
    }

    fn set_uniform_fvec4(&self, name: &String, value: glam::Vec4) {
        let uniform_data = self
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Float4,
            "Wrong uniform type"
        );
        get_context!(context);
        let gl = &context.gl;

        self.bind();
        unsafe {
            gl.uniform_4_f32(Some(&uniform_data.location), value.x, value.y, value.z, value.w);
        }
    }

    fn set_uniform_i32(&self, name: &String, value: i32) {
        let uniform_data = self
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Int,
            "Wrong uniform type"
        );
        get_context!(context);
        let gl = &context.gl;

        self.bind();
        unsafe {
            gl.uniform_1_i32(Some(&uniform_data.location), value);
        }
    }
}

/// Compile shaders into a program. The map goes from shader type (fragment, vertex)
/// to the shader code
fn compile_shaders(shaders: Vec<(u32, String)>) -> Result<NativeProgram, ShaderError> {
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
            gl.shader_source(shader, &source.as_str());
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

        return Ok(program);
    }
}
