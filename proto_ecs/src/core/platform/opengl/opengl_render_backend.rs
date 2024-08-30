use glow::{Context, HasContext};
use lazy_static::lazy_static;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::rendering::render_api::{
    RenderAPIBackend, RenderAPIBackendDyn, RenderAPIBackendPtr,
};
use proto_ecs::core::windowing::window_manager;

use crate::core::math::Colorf32;
use crate::core::platform::opengl::opengl_buffer::{OpenGLIndexBuffer, OpenGLVertexBuffer};
use crate::core::platform::opengl::opengl_shader::{compile_shaders, OpenGLShader, UniformData};
use crate::core::platform::opengl::opengl_vertex_array::OpenGLVertexArray;
use crate::core::platform::winit_window::WinitWindow;
use crate::core::rendering::buffer::BufferLayout;
use crate::core::rendering::handle::Allocator;
use crate::core::rendering::render_api::API;
use crate::core::rendering::render_api::{
    IndexBufferHandle, ShaderHandle, VertexArrayHandle, VertexBufferHandle,
};
use crate::core::rendering::shader::{ShaderDataType, ShaderError};

use std::collections::HashMap;
use std::mem::size_of;

pub(super) struct OpenGLContext {
    pub(super) gl: Context,
}

// TODO implementar Send + Sync
unsafe impl Send for OpenGLContext {}
unsafe impl Sync for OpenGLContext {}

pub struct OpenGLRenderBackend {
    pub(super) clear_color: Colorf32,
    shader_allocator: Allocator<OpenGLShader>,
    vertex_array_allocator: Allocator<OpenGLVertexArray>,
    index_buffer_allocator: Allocator<OpenGLIndexBuffer>,
    vertex_buffer_allocator: Allocator<OpenGLVertexBuffer>,
}

lazy_static! {
    pub(super) static ref OPENGL_CONTEXT: RwLock<Option<OpenGLContext>> = RwLock::new(None);
}

/// Simple macro that should NOT be used outside this module, it's just used
/// as a shortcut to get a reference to the opengl context
macro_rules! get_context {
    ($i:ident) => {
        let __context__ =
            proto_ecs::core::platform::opengl::opengl_render_backend::OPENGL_CONTEXT.read();
        let $i = __context__
            .as_ref()
            .expect("Opengl Context not yet initialized!");
    };
}

pub(super) use get_context;

impl RenderAPIBackend for OpenGLRenderBackend {
    fn create() -> RenderAPIBackendPtr {
        // We have to get a reference to the opengl context created by winit
        let window_manager = window_manager::WindowManager::get().read();
        let winit_window = window_manager
            .get_window()
            .as_any()
            .downcast_ref::<WinitWindow>()
            .expect("The OpenGL render backend is only compatible with WinitWindow windows");
        {
            let mut context = OPENGL_CONTEXT.write();
            debug_assert!(context.is_none(), "Already existent OpenGL api backend");
            *context = Some(OpenGLContext {
                gl: winit_window.get_glow_context(),
            });
        }

        let mut result = Box::new(OpenGLRenderBackend {
            clear_color: Colorf32::new(0.0, 0.0, 0.0, 1.0),
            shader_allocator: Allocator::new(),
            vertex_array_allocator: Allocator::new(),
            index_buffer_allocator: Allocator::new(),
            vertex_buffer_allocator: Allocator::new(),
        });
        result.init();
        result
    }
}

impl RenderAPIBackendDyn for OpenGLRenderBackend {
    fn clear_color(&self) {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.clear(glow::COLOR_BUFFER_BIT);
        };
    }

    fn draw_indexed(&mut self, vertex_array: VertexArrayHandle) {
        // Assume that vertex array is bound right now
        self.bind_vertex_array(vertex_array);
        let vertex_array = self.vertex_array_allocator.get(vertex_array);

        unsafe {
            let count = self.get_index_buffer_count(
                vertex_array
                    .index_buffer
                    .expect("Can't draw-indexed over array with no index"),
            ) as i32;

            get_context!(context);
            context
                .gl
                .draw_elements(glow::TRIANGLES, count, glow::UNSIGNED_INT, 0);
        }
    }

    fn get_api(&self) -> API {
        API::OpenGL
    }

    fn init(&mut self) {
        println!("Glow OpenGL successfully initialized!");
        let opengl_version = self.get_string(glow::VERSION);
        let opengl_renderer = self.get_string(glow::RENDERER);
        let opengl_vendor = self.get_string(glow::VENDOR);

        println!("\tOpenGL Version: {}", opengl_version);
        println!("\tOpenGL Renderer: {}", opengl_renderer);
        println!("\tOpenGL Vendor: {}", opengl_vendor);
    }

    fn set_clear_color(&mut self, color: Colorf32) {
        self.clear_color = color;
        get_context!(context);
        unsafe {
            context.gl.clear_color(
                self.clear_color.x,
                self.clear_color.y,
                self.clear_color.z,
                self.clear_color.w,
            );
        }
    }

    fn set_viewport(&mut self, x: u32, y: u32, width: u32, height: u32) {
        get_context!(context);
        unsafe {
            context
                .gl
                .viewport(x as i32, y as i32, width as i32, height as i32);
        }
    }

    // Resource creation and destruction
    fn create_vertex_buffer(&mut self, vertex_data: &[f32]) -> VertexBufferHandle {
        get_context!(context);
        let gl = &context.gl;

        unsafe {
            // TODO Better error handling
            let native_buffer = gl.create_buffer().expect("Could not create vertex buffer");
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(native_buffer));
            let bytes = std::slice::from_raw_parts(
                vertex_data.as_ptr().cast::<u8>(),
                vertex_data.len() * (size_of::<f32>() / size_of::<u8>()),
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);

            self.vertex_buffer_allocator.allocate(OpenGLVertexBuffer {
                native_buffer,
                buffer_layout: BufferLayout::default(),
            })
        }
    }
    fn destroy_vertex_buffer(&mut self, handle: VertexBufferHandle) {
        get_context!(context);
        let gl = &context.gl;
        let buffer = self.vertex_buffer_allocator.get(handle);

        unsafe { gl.delete_buffer(buffer.native_buffer) }

        self.vertex_buffer_allocator.free(handle);
    }
    fn create_index_buffer(&mut self, indices: &[u32]) -> IndexBufferHandle {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            // TODO Better error handling would be nice
            let buffer_id = gl.create_buffer().expect("Unable to create index buffer");

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(buffer_id));
            let u8_slice = std::slice::from_raw_parts(
                indices.as_ptr().cast::<u8>(),
                // kind of unnecessary since u32 and u8 have 4 bytes and 1 byte by definition
                indices.len() * (size_of::<u32>() / size_of::<u8>()),
            );
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, u8_slice, glow::STATIC_DRAW);

            self.index_buffer_allocator.allocate(OpenGLIndexBuffer {
                native_buffer: buffer_id,
                element_count: indices.len(),
            })
        }
    }
    fn destroy_index_buffer(&mut self, handle: IndexBufferHandle) {
        get_context!(context);
        let gl = &context.gl;
        let index_buffer = self.index_buffer_allocator.get(handle);

        unsafe {
            gl.delete_buffer(index_buffer.native_buffer);
        }

        self.index_buffer_allocator.free(handle);
    }
    fn create_vertex_array(&mut self) -> VertexArrayHandle {
        get_context!(context);
        let gl = &context.gl;
        let native_array = unsafe {
            gl.create_vertex_array()
                .expect("Could not create OpenGL vertex array")
        };

        self.vertex_array_allocator.allocate(OpenGLVertexArray {
            native_array,
            vertex_buffer: None,
            index_buffer: None,
        })
    }
    fn destroy_vertex_array(&mut self, handle: VertexArrayHandle) {
        get_context!(context);
        let gl = &context.gl;
        let vertex_array = self.vertex_array_allocator.get(handle);
        unsafe {
            gl.delete_vertex_array(vertex_array.native_array);
        }
        self.vertex_array_allocator.free(handle);
    }
    fn create_shader(
        &mut self,
        name: &str,
        vertex_src: &str,
        fragment_src: &str,
    ) -> Result<ShaderHandle, ShaderError> {
        let shaders = vec![
            (glow::VERTEX_SHADER, vertex_src),
            (glow::FRAGMENT_SHADER, fragment_src),
        ];
        let uniforms = HashMap::new();

        let program = compile_shaders(shaders)?;
        let new_shader = self.shader_allocator.allocate(OpenGLShader {
            name: name.to_string(),
            native_program: program,
            uniforms,
        });

        Ok(new_shader)
    }
    fn destroy_shader(&mut self, handle: ShaderHandle) {
        get_context!(context);
        debug_assert!(
            self.shader_allocator.is_live(handle),
            "Trying to destroy unexistent shader"
        );
        let shader = self.shader_allocator.get(handle);

        unsafe {
            context.gl.delete_program(shader.native_program);
        }
        self.shader_allocator.free(handle);
    }

    // Bindings
    fn bind_vertex_buffer(&self, handle: VertexBufferHandle) {
        get_context!(context);
        let gl = &context.gl;
        let vertex_buffer = self.vertex_buffer_allocator.get(handle);
        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertex_buffer.native_buffer));
        }
    }
    fn unbind_vertex_buffer(&self) {
        get_context!(context);
        let gl = &context.gl;

        unsafe {
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }
    fn bind_vertex_array(&self, handle: VertexArrayHandle) {
        get_context!(context);
        let gl = &context.gl;
        let vertex_array = self.vertex_array_allocator.get(handle);
        unsafe {
            gl.bind_vertex_array(Some(vertex_array.native_array));
        }
        if let Some(ib) = vertex_array.index_buffer {
            self.bind_index_buffer(ib);
        }
        if let Some(vb) = vertex_array.vertex_buffer {
            self.bind_vertex_buffer(vb);
        }
    }
    fn unbind_vertex_array(&self) {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.bind_vertex_array(None);
        }
    }
    fn bind_index_buffer(&self, handle: IndexBufferHandle) {
        get_context!(context);
        let gl = &context.gl;
        let index_buffer = self.index_buffer_allocator.get(handle);
        unsafe {
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(index_buffer.native_buffer));
        }
    }
    fn unbind_index_buffer(&self) {
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);
        }
    }
    fn bind_shader(&self, handle: ShaderHandle) {
        get_context!(context);
        let gl = &context.gl;
        let shader = self.shader_allocator.get(handle);

        unsafe {
            gl.use_program(Some(shader.native_program));
        }
    }
    fn unbind_shader(&self) {
        get_context!(context);
        let gl = &context.gl;

        unsafe {
            gl.use_program(None);
        }
    }

    // Operations: Index buffer
    fn get_index_buffer_count(&self, handle: IndexBufferHandle) -> u32 {
        let index_buffer = self.index_buffer_allocator.get(handle);
        index_buffer.element_count as u32
    }

    // Operations: Vertex Buffer
    fn get_vertex_buffer_layout(&self, handle: VertexBufferHandle) -> &BufferLayout {
        let vertex_buffer = self.vertex_buffer_allocator.get(handle);
        &vertex_buffer.buffer_layout
    }
    fn set_vertex_buffer_layout(&self, handle: VertexBufferHandle, layout: BufferLayout) {
        let vertex_buffer = self.vertex_buffer_allocator.get(handle);
        vertex_buffer.buffer_layout = layout;
    }

    // Operations: Vertex Array
    fn set_vertex_array_vertex_buffer(
        &mut self,
        va_handle: VertexArrayHandle,
        vb_handle: VertexBufferHandle,
    ) {
        self.bind_vertex_array(va_handle);
        self.bind_vertex_buffer(vb_handle);
        let vertex_buffer = self.vertex_buffer_allocator.get(vb_handle);

        let layout = vertex_buffer.get_buffer_layout();
        {
            get_context!(context);
            let gl = &context.gl;
            for (i, element) in layout.iter().enumerate() {
                unsafe {
                    gl.enable_vertex_attrib_array(i as u32);
                    let element_count = element.get_component_count();
                    match element.get_data_type() {
                        ShaderDataType::Float_32
                        | ShaderDataType::Float2_32
                        | ShaderDataType::Float3_32
                        | ShaderDataType::Float4_32
                        | ShaderDataType::Mat3_32
                        | ShaderDataType::Mat4_32 => {
                            gl.vertex_attrib_pointer_f32(
                                i as u32,
                                element_count as i32,
                                glow::FLOAT,
                                element.is_normalized(),
                                layout.get_stride() as i32,
                                element.get_offset() as i32,
                            );
                        }
                        ShaderDataType::Int_32
                        | ShaderDataType::Int2_32
                        | ShaderDataType::Int3_32
                        | ShaderDataType::Int4_32
                        | ShaderDataType::Bool => gl.vertex_attrib_pointer_i32(
                            i as u32,
                            element_count as i32,
                            glow::INT,
                            layout.get_stride() as i32,
                            element.get_offset() as i32,
                        ),
                        _ => panic!("Don't know how define attribute of this type"),
                    }
                }
            }
        }
        self.unbind_vertex_buffer();
        let vertex_array = self.vertex_array_allocator.get(va_handle);
        vertex_array.vertex_buffer = Some(vb_handle);
    }
    fn set_vertex_array_index_buffer(
        &mut self,
        va_handle: VertexArrayHandle,
        ib_handle: IndexBufferHandle,
    ) {
        let va = self.vertex_array_allocator.get(va_handle);
        va.index_buffer = Some(ib_handle);
    }
    fn get_vertex_array_vertex_buffer(
        &self,
        va_handle: VertexArrayHandle,
    ) -> Option<VertexBufferHandle> {
        let va = self.vertex_array_allocator.get(va_handle);
        va.vertex_buffer
    }
    fn get_vertex_array_index_buffer(
        &self,
        va_handle: VertexArrayHandle,
    ) -> Option<IndexBufferHandle> {
        let va = self.vertex_array_allocator.get(va_handle);
        va.index_buffer
    }

    // Operations: Shaders
    fn get_shader_name(&self, handle: ShaderHandle) -> &str {
        let shader = self.shader_allocator.get(handle);
        &shader.name
    }
    fn set_shader_uniform_f32(&mut self, handle: ShaderHandle, name: &str, value: f32) {
        let shader = self.shader_allocator.get(handle);
        let uniform_data = shader
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Float_32,
            "Wrong uniform type"
        );
        get_context!(context);
        let gl = &context.gl;

        self.bind_shader(handle);
        unsafe {
            gl.uniform_1_f32(Some(&uniform_data.location), value);
        }
    }
    fn set_shader_uniform_i32(&mut self, handle: ShaderHandle, name: &str, value: i32) {
        let shader = self.shader_allocator.get(handle);
        let uniform_data = shader
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Int_32,
            "Wrong uniform type"
        );

        self.bind_shader(handle);
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.uniform_1_i32(Some(&uniform_data.location), value);
        }
    }
    fn set_shader_uniform_fvec2(&mut self, handle: ShaderHandle, name: &str, value: &glam::Vec2) {
        let shader = self.shader_allocator.get(handle);
        let uniform_data = shader
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Float2_32,
            "Wrong uniform type"
        );

        self.bind_shader(handle);
        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.uniform_2_f32(Some(&uniform_data.location), value.x, value.y);
        }
    }
    fn set_shader_uniform_fvec3(&mut self, handle: ShaderHandle, name: &str, value: &glam::Vec3) {
        let shader = self.shader_allocator.get(handle);
        let uniform_data = shader
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Float3_32,
            "Wrong uniform type"
        );

        self.bind_shader(handle);
        unsafe {
            get_context!(context);
            let gl = &context.gl;
            gl.uniform_3_f32(Some(&uniform_data.location), value.x, value.y, value.z);
        }
    }
    fn set_shader_uniform_fvec4(&mut self, handle: ShaderHandle, name: &str, value: &glam::Vec4) {
        let shader = self.shader_allocator.get(handle);
        let uniform_data = shader
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Float4_32,
            "Wrong uniform type"
        );
        get_context!(context);
        let gl = &context.gl;

        self.bind_shader(handle);
        unsafe {
            gl.uniform_4_f32(
                Some(&uniform_data.location),
                value.x,
                value.y,
                value.z,
                value.w,
            );
        }
    }
    fn set_shader_uniform_fmat3(&mut self, handle: ShaderHandle, name: &str, value: &glam::Mat3) {
        let shader = self.shader_allocator.get(handle);
        let uniform_data = shader
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Mat3_32,
            "Wrong uniform type"
        );

        self.bind_shader(handle);
        unsafe {
            get_context!(context);
            let gl = &context.gl;
            gl.uniform_matrix_3_f32_slice(
                Some(&uniform_data.location),
                false,
                value.as_ref().as_slice(),
            );
        }
    }
    fn set_shader_uniform_fmat4(&mut self, handle: ShaderHandle, name: &str, value: &glam::Mat4) {
        let shader = self.shader_allocator.get(handle);
        let uniform_data = shader
            .uniforms
            .get(name)
            .expect("Trying to access unexistent uniform");
        debug_assert!(
            uniform_data.data_type == ShaderDataType::Mat3_32,
            "Wrong uniform type"
        );

        self.bind_shader(handle);

        get_context!(context);
        let gl = &context.gl;
        unsafe {
            gl.uniform_matrix_3_f32_slice(
                Some(&uniform_data.location),
                false,
                value.as_ref().as_slice(),
            );
        }
    }
    fn add_shader_uniform(
        &mut self,
        handle: ShaderHandle,
        name: &str,
        data_type: ShaderDataType,
    ) -> Result<(), ShaderError> {
        let shader = self.shader_allocator.get(handle);

        if let Some(uniform_data) = shader.uniforms.get(name) {
            return Err(ShaderError::UniformAlreadyExists {
                uniform_name: name.to_string(),
                prev_type: uniform_data.data_type,
            });
        };
        get_context!(context);
        let gl = &context.gl;
        let location = unsafe {
            gl.get_uniform_location(shader.native_program, name)
                .unwrap_or_else(
                    || panic!(
                        "Could not get an attribute location for shader '{}'. Did you forget to USE the uniform in that shader?",
                        shader.name
                    )
                )
        };
        shader.uniforms.insert(
            name.to_string(),
            UniformData {
                data_type,
                location,
            },
        );
        Ok(())
    }
}

impl OpenGLRenderBackend {
    #[inline(always)]
    fn get_string(&self, variant: u32) -> String {
        get_context!(context);
        unsafe { context.gl.get_parameter_string(variant) }
    }
}
