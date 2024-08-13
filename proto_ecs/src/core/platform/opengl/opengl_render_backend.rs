use glow::{Context, HasContext};
use lazy_static::lazy_static;
use proto_ecs::core::locking::RwLock;
use proto_ecs::core::rendering::render_api::{
    RenderAPIBackend, RenderAPIBackendDyn, RenderAPIBackendPtr,
};
use proto_ecs::core::windowing::window_manager;

use crate::core::math::Color;
use crate::core::platform::winit_window::WinitWindow;
use crate::core::rendering::render_api::API;
use crate::core::rendering::vertex_array::VertexArrayDyn;

pub(super) struct OpenGLContext {
    pub(super) gl: Context,
}

// TODO implementar bien la sincronizacion de este objeto
unsafe impl Send for OpenGLContext {}
unsafe impl Sync for OpenGLContext {}

pub struct OpenGLRenderBackend {
    pub(super) clear_color: Color,
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
            clear_color: Color::new(0.0, 0.0, 0.0, 1.0),
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

    fn draw_indexed(&mut self, vertex_array: &dyn VertexArrayDyn) {
        // Assume that vertex array is bound right now
        get_context!(context);
        vertex_array.bind();
        unsafe {
            let count = vertex_array
                .get_index_buffer()
                .as_ref()
                .expect("Should have index buffer by now")
                .get_count() as i32;

            context.gl.draw_arrays(glow::TRIANGLES, 0, count);
            // context.gl.draw_elements(
            //     glow::TRIANGLES,
            //     vertex_array
            //         .get_index_buffer()
            //         .as_ref()
            //         .expect("Should have index buffer by now")
            //         .get_count() as i32,
            //     glow::UNSIGNED_INT,
            //     0,
            // );
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

    fn set_clear_color(&mut self, color: Color) {
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
}

impl OpenGLRenderBackend {
    #[inline(always)]
    fn get_string(&self, variant: u32) -> String {
        get_context!(context);
        unsafe { context.gl.get_parameter_string(variant) }
    }
}
